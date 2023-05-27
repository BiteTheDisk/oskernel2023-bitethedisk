use super::chunk_area::ChunkArea;
use super::{MapArea, MapPermission, MapType};
use crate::consts::{PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_HEAP_SIZE, USER_STACK_SIZE};
use crate::fs::OSInode;
use crate::mm::frame_allocator::{enquire_refcount, frame_add_ref};
use crate::mm::page_table::PTEFlags;
use crate::mm::{
    alloc_frame, FrameTracker, PageTable, PageTableEntry, PhysAddr, PhysPageNum, VirtAddr,
    VirtPageNum,
};
use alloc::{sync::Arc, vec::Vec};

pub struct MemorySet {
    pub page_table: PageTable,

    vm_areas: Vec<MapArea>,

    mmap_areas: Vec<ChunkArea>,

    heap_areas: ChunkArea,

    pub brk_start: usize,
    pub brk: usize,
}

impl MemorySet {
    /// 新建一个空的地址空间
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            vm_areas: Vec::new(),
            heap_areas: ChunkArea::new(
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
                0.into(),
                0.into(),
            ),
            mmap_areas: Vec::new(),
            brk_start: 0,
            brk: 0,
        }
    }

    /// 获取当前页表的 token (符合 satp CSR 格式要求的多级页表的根节点所在的物理页号)
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// 在当前地址空间插入一个 `Framed` 方式映射到物理内存的逻辑段
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    pub fn remove_mmap_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((idx, chunk)) = self
            .mmap_areas
            .iter_mut()
            .enumerate()
            .find(|(_, chunk)| chunk.vpn_range.get_start() == start_vpn)
        {
            chunk.unmap(&mut self.page_table);
            self.mmap_areas.remove(idx);
        }
    }

    /// 通过起始虚拟页号删除对应的逻辑段（包括连续逻辑段和离散逻辑段）
    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((idx, area)) = self
            .vm_areas
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.vpn_range.get_start() == start_vpn)
        {
            area.unmap(&mut self.page_table);
            self.vm_areas.remove(idx);
        }
    }
    /// 在当前地址空间插入一个新的连续逻辑段
    ///
    /// - 物理页号是随机分配的
    /// - 如果是以 Framed 方式映射到物理内存,
    /// 还可以可选性地在那些被映射到的物理页帧上写入一些初始化数据
    /// - data:(osinode,offset,len,page_offset)
    pub fn push(
        &mut self,
        mut map_area: MapArea,
        data: Option<(Arc<OSInode>, usize, usize, usize)>,
    ) {
        // println!("[KERNEL] push maparea start {:?}", map_area.start_va);
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            // 写入初始化数据，如果数据存在
            map_area.copy_data(&mut self.page_table, data.0, data.1, data.2, data.3);
        }
        self.vm_areas.push(map_area); // 将生成的数据段压入 areas 使其生命周期由areas控制
    }

    /// 在当前地址空间插入一段已被分配空间的连续逻辑段
    ///
    /// 主要用于 COW 创建时子进程空间连续逻辑段的插入，其要求指定物理页号
    fn push_mapped_area(&mut self, map_area: MapArea) {
        self.vm_areas.push(map_area);
    }

    /// 映射跳板的虚拟页号和物理物理页号
    pub fn map_trampoline(&mut self) {
        extern "C" {
            fn strampoline();
        }
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    fn map_trap_context(&mut self) {
        self.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
    }

    /// ### 从 ELF 格式可执行文件解析出各数据段并对应生成应用的地址空间
    pub fn load_elf(elf_file: Arc<OSInode>) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();

        // 将跳板插入到应用地址空间
        memory_set.map_trampoline();

        // 在应用地址空间中映射次高页面来存放 Trap 上下文
        // 将 TRAP_CONTEXT 段尽量放前，以节省 cow 时寻找时间
        memory_set.map_trap_context();

        // 第一次读取前64字节确定程序表的位置与大小
        let elf_head_data = elf_file.read_vec(0, 64);
        let elf_head_data_slice = elf_head_data.as_slice();
        let elf = xmas_elf::ElfFile::new(elf_head_data_slice).unwrap();

        let ph_entry_size = elf.header.pt2.ph_entry_size() as usize;
        let ph_offset = elf.header.pt2.ph_offset() as usize;
        let ph_count = elf.header.pt2.ph_count() as usize;

        // 进行第二次读取，这样的elf对象才能正确解析程序段头的信息
        let elf_head_data = elf_file.read_vec(0, ph_offset + ph_count * ph_entry_size);
        let elf = xmas_elf::ElfFile::new(elf_head_data.as_slice()).unwrap();

        // 记录目前涉及到的最大的虚拟页号
        let mut max_end_vpn = VirtPageNum(0);

        // 遍历程序段进行加载
        for i in 0..ph_count as u16 {
            let ph = elf.program_header(i).unwrap();
            match ph.get_type().unwrap() {
                xmas_elf::program::Type::Load => {
                    let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                    let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                    let mut map_perm = MapPermission::U;
                    let ph_flags = ph.flags();
                    if ph_flags.is_read() {
                        map_perm |= MapPermission::R;
                    }
                    if ph_flags.is_write() {
                        map_perm |= MapPermission::W;
                    }
                    if ph_flags.is_execute() {
                        map_perm |= MapPermission::X;
                    }
                    let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                    max_end_vpn = map_area.vpn_range.get_end();
                    memory_set.push(
                        map_area,
                        Some((
                            elf_file.clone(),
                            ph.offset() as usize,
                            ph.file_size() as usize,
                            start_va.page_offset(),
                        )),
                    );
                }
                _ => continue,
            }
        }

        // 分配用户栈
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into(); // 栈底
        user_stack_bottom += PAGE_SIZE; // 在已用最大虚拟页之上放置一个保护页
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE; // 栈顶地址
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        // 分配用户堆，lazy加载
        let mut user_heap_bottom: usize = user_stack_top;
        user_heap_bottom += PAGE_SIZE; //放置一个保护页
        let user_heap_top: usize = user_heap_bottom + USER_HEAP_SIZE;
        memory_set.heap_areas = ChunkArea::new(
            MapType::Framed,
            MapPermission::R | MapPermission::W | MapPermission::U,
            user_heap_bottom.into(),
            user_heap_top.into(),
        );

        memory_set.brk = user_heap_bottom;
        memory_set.brk_start = user_heap_bottom;

        (
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        )
    }

    /// 以COW的方式复制一个地址空间
    pub fn from_copy_on_write(user_space: &mut MemorySet) -> MemorySet {
        let mut new_memory_set = Self::new_bare(); // use 1 page (page_table root)

        // This part is not for Copy on Write.
        // Including:   Trampoline
        //              Trap_Context
        new_memory_set.map_trampoline(); // use 2 pages (page_table create ptes)
        for area in user_space.vm_areas.iter() {
            // use 1 page
            let start_vpn = area.vpn_range.get_start();
            if start_vpn == VirtAddr::from(TRAP_CONTEXT).floor() {
                let new_area = MapArea::from_another(area);
                new_memory_set.push(new_area, None);
                for vpn in area.vpn_range {
                    let src_ppn = user_space.translate(vpn).unwrap().ppn();
                    let dst_ppn = new_memory_set.translate(vpn).unwrap().ppn();
                    // println!{"[COW TRAP_CONTEXT] mapping {:?} --- {:?}, src: {:?}", vpn, dst_ppn, src_ppn};
                    dst_ppn
                        .as_bytes_array()
                        .copy_from_slice(src_ppn.as_bytes_array());
                }
            }
            break;
        }
        // This part is for copy on write
        let parent_areas = &user_space.vm_areas;
        let parent_page_table = &mut user_space.page_table;
        for area in parent_areas.iter() {
            let start_vpn = area.vpn_range.get_start();
            if start_vpn != VirtAddr::from(TRAP_CONTEXT).floor() {
                let mut new_area = MapArea::from_another(area);
                // map the former physical address
                for vpn in area.vpn_range {
                    // change the map permission of both pagetable
                    // get the former flags and ppn
                    let pte = parent_page_table.translate(vpn).unwrap();
                    let pte_flags = pte.flags() & !PTEFlags::W;
                    let src_ppn = pte.ppn();
                    frame_add_ref(src_ppn);
                    // change the flags of the src_pte
                    parent_page_table.set_flags(vpn, pte_flags);
                    parent_page_table.set_cow(vpn);
                    // map the cow page table to src_ppn
                    new_memory_set.page_table.map(vpn, src_ppn, pte_flags);
                    new_memory_set.page_table.set_cow(vpn);
                    new_area.data_frames.push(FrameTracker::from_ppn(src_ppn));
                }
                new_memory_set.push_mapped_area(new_area);
            }
        }
        for chunk in user_space.mmap_areas.iter() {
            let mut new_chunk = ChunkArea::from_another(chunk);
            for vpn in chunk.vpn_table.iter() {
                let vpn = vpn.clone();
                // change the map permission of both pagetable
                // get the former flags and ppn
                let pte = parent_page_table.translate(vpn).unwrap();
                let pte_flags = pte.flags() & !PTEFlags::W;
                let src_ppn = pte.ppn();
                frame_add_ref(src_ppn);
                // change the flags of the src_pte
                parent_page_table.set_flags(vpn, pte_flags);
                parent_page_table.set_cow(vpn);
                // map the cow page table to src_ppn
                new_memory_set.page_table.map(vpn, src_ppn, pte_flags);
                new_memory_set.page_table.set_cow(vpn);
                new_chunk.vpn_table.push(vpn);
                new_chunk.data_frames.push(FrameTracker::from_ppn(src_ppn));
            }
            new_memory_set.mmap_areas.push(new_chunk);
        }
        new_memory_set.heap_areas = ChunkArea::from_another(&user_space.heap_areas);
        for _vpn in user_space.heap_areas.vpn_table.iter() {
            let vpn = (*_vpn).clone();
            // change the map permission of both pagetable
            // get the former flags and ppn
            let pte = parent_page_table.translate(vpn).unwrap();
            let pte_flags = pte.flags() & !PTEFlags::W;
            let src_ppn = pte.ppn();
            frame_add_ref(src_ppn);
            // change the flags of the src_pte
            parent_page_table.set_flags(vpn, pte_flags);
            parent_page_table.set_cow(vpn);
            // map the cow page table to src_ppn
            new_memory_set.page_table.map(vpn, src_ppn, pte_flags);
            new_memory_set.page_table.set_cow(vpn);
            new_memory_set.heap_areas.vpn_table.push(vpn);
            new_memory_set
                .heap_areas
                .data_frames
                .push(FrameTracker::from_ppn(src_ppn));
        }
        new_memory_set.brk_start = user_space.brk_start;
        new_memory_set.brk = user_space.brk;
        new_memory_set
    }

    #[no_mangle]
    pub fn cow_alloc(&mut self, vpn: VirtPageNum, former_ppn: PhysPageNum) -> isize {
        if enquire_refcount(former_ppn) == 1 {
            self.page_table.reset_cow(vpn);
            // change the flags of the src_pte
            self.page_table.set_flags(
                vpn,
                self.page_table.translate(vpn).unwrap().flags() | PTEFlags::W,
            );
            return 0;
        }
        let frame = alloc_frame().unwrap();
        let ppn = frame.ppn;
        self.remap_cow(vpn, ppn, former_ppn);
        for area in self.vm_areas.iter_mut() {
            let head_vpn = area.vpn_range.get_start();
            let tail_vpn = area.vpn_range.get_end();
            if vpn < tail_vpn && vpn >= head_vpn {
                area.data_frames.push(frame);
                return 0;
            }
        }
        for chunk in self.mmap_areas.iter_mut() {
            let head_vpn = chunk.vpn_range.get_start();
            let tail_vpn = chunk.vpn_range.get_end();
            if vpn < tail_vpn && vpn >= head_vpn {
                chunk.data_frames.push(frame);
                return 0;
            }
        }
        let head_vpn = self.heap_areas.vpn_range.get_start();
        let tail_vpn = self.heap_areas.vpn_range.get_end();
        if vpn < tail_vpn && vpn >= head_vpn {
            self.heap_areas.data_frames.push(frame);
            return 0;
        }
        0
    }

    fn remap_cow(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, former_ppn: PhysPageNum) {
        self.page_table.remap_cow(vpn, ppn, former_ppn);
    }

    /// 为mmap缺页分配空页表
    pub fn lazy_mmap(&mut self, stval: VirtAddr) -> isize {
        for mmap_chunk in self.mmap_areas.iter_mut() {
            if stval >= mmap_chunk.vpn_range.get_start().into()
                && stval < mmap_chunk.vpn_range.get_end().into()
            {
                mmap_chunk.push_vpn(stval.floor(), &mut self.page_table);
                return 0;
            }
        }
        -1
    }

    pub fn lazy_alloc_heap(&mut self, vpn: VirtPageNum) -> isize {
        self.heap_areas.push_vpn(vpn, &mut self.page_table);

        0
    }

    /// 根据多级页表和 vpn 查找页表项
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    /// 回收应用地址空间
    ///
    /// 将地址空间中的逻辑段列表 areas 清空（即执行 Vec 向量清空），
    /// 这将导致应用地址空间被回收（即进程的数据和代码对应的物理页帧都被回收），
    /// 但用来存放页表的那些物理页帧此时还不会被回收（会由父进程最后回收子进程剩余的占用资源）
    pub fn recycle_data_pages(&mut self) {
        //*self = Self::new_bare();
        self.vm_areas.clear();
    }

    /// 在地址空间中插入一个空的离散逻辑段
    ///
    /// - 已确定：
    ///     - 起止虚拟地址
    ///     - 映射方式：Framed
    ///     - map_perm
    /// - 留空：
    ///     - vpn_table
    ///     - data_frames
    pub fn insert_mmap_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        let new_chunk_area = ChunkArea::new(MapType::Framed, permission, start_va, end_va);

        self.mmap_areas.push(new_chunk_area);
    }

    pub fn check_va_range(&self, start_va: VirtAddr, len: usize) -> bool {
        let end_va = VirtAddr::from(start_va.0 + len);
        for area in self.vm_areas.iter() {
            if area.vpn_range.get_start() <= start_va.floor()
                && end_va.ceil() <= area.vpn_range.get_end()
            {
                return true;
            }
        }
        for chunk in self.mmap_areas.iter() {
            if chunk.vpn_range.get_start() <= start_va.floor()
                && end_va.ceil() <= chunk.vpn_range.get_end()
            {
                return true;
            }
        }
        if self.heap_areas.vpn_range.get_start() <= start_va.floor()
            && end_va.ceil() <= self.heap_areas.vpn_range.get_end()
        {
            return true;
        }
        return false;
    }
}
