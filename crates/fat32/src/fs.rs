use core::assert;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::clone::Clone;
use core::option::Option;
use core::option::Option::{None, Some};
use spin::RwLock;

use super::bpb::{BIOSParameterBlock, BasicBPB, FSInfo, BPB32};
use super::cache::get_block_cache;
use super::cache::Cache;
use super::device::BlockDevice;
use super::entry::ShortDirEntry;
use super::fat::FATManager;
use super::vf::VirtFileType;
use super::{BLOCK_NUM, BLOCK_SIZE, END_OF_CLUSTER, FREE_CLUSTER, ROOT};
use crate::ROOT_DIR_CLUSTER;

pub struct FileSystem {
    pub(crate) device: Arc<dyn BlockDevice>,
    pub(crate) free_cluster_cnt: Arc<RwLock<usize>>,
    pub(crate) bpb: BIOSParameterBlock, // read only
    pub(crate) fat: Arc<RwLock<FATManager>>,
    pub(crate) root_dir_entry: Arc<RwLock<ShortDirEntry>>, // 虚拟根目录项。根目录无目录项，引入以与其他文件一致
}

impl FileSystem {
    pub fn sector_pre_cluster(&self) -> usize {
        self.bpb.sector_per_cluster()
    }
    pub fn sector_size(&self) -> usize {
        self.bpb.bytes_per_sector()
    }
    pub fn cluster_size(&self) -> usize {
        self.bpb.bytes_per_sector() * self.bpb.sector_per_cluster()
    }
    pub fn first_data_sector(&self) -> usize {
        self.bpb.first_data_sector()
    }
    pub fn free_cluster_cnt(&self) -> usize {
        *self.free_cluster_cnt.read()
    }
    pub fn set_free_clusters(&self, cnt: usize) {
        get_block_cache(self.bpb.fat_info_sector(), Arc::clone(&self.device))
            .write()
            .modify(0, |fsinfo: &mut FSInfo| {
                fsinfo.set_free_clusters(cnt as u32)
            });
        *self.free_cluster_cnt.write() = cnt;
    }
    pub fn first_sector_of_cluster(&self, cluster: u32) -> usize {
        self.bpb.first_sector_of_cluster(cluster)
    }
    pub fn cluster_offset(&self, cluster: u32) -> usize {
        self.bpb.offset(cluster)
    }
    pub fn root_sector_id(&self) -> usize {
        self.first_data_sector()
    }
    #[allow(unused)]
    // only for std test
    pub fn create(device: Arc<dyn BlockDevice>) -> Arc<RwLock<Self>> {
        let basic_bpb = BasicBPB {
            _bs_jmp_boot: [0xEB, 0x58, 0x90],
            _bs_oem_name: *b"mk.fat32",
            byts_per_sec: BLOCK_SIZE as u16,
            sec_per_clus: 8,
            rsvd_sec_cnt: 32,
            num_fats: 2,
            root_ent_cnt: 0,
            tot_sec16: 0,
            _media: 0xF8,
            fat_sz16: 0,
            _sec_per_trk: 0,
            _num_heads: 0,
            _hidd_sec: 0,
            tot_sec32: 0x4000 as u32,
        };
        let bpb32 = BPB32 {
            fat_sz32: 64,
            _ext_flags: 0,
            _fs_ver: 0,
            root_clus: ROOT_DIR_CLUSTER,
            fs_info: 1,
            _bk_boot_sec: 6,
            _reserved: [0u8; 12],
            _bs_drv_num: 0x80,
            _bs_reserved1: 0,
            _bs_boot_sig: 0x29,
            _bs_vol_id: 0x12345678,
            _bs_vol_lab: *b"mkfs.fat32 ",
            _bs_fil_sys_type: *b"FAT32   ",
        };
        let bpb = BIOSParameterBlock { basic_bpb, bpb32 };
        get_block_cache(0, Arc::clone(&device))
            .write()
            .modify(0, |b: &mut BIOSParameterBlock| *b = bpb);
        let fsinfo = FSInfo {
            lead_sig: 0x41615252,
            _reserved1: [0u8; 480],
            struc_sig: 0x61417272,
            free_count: BLOCK_NUM as u32 - 32 - 128 - 128,
            nxt_free: 0xFFFFFFFF,
            _reserved2: [0u8; 12],
            trail_sig: 0xAA550000,
        };
        let free_cluster_cnt = fsinfo.free_cluster_cnt() as usize;
        get_block_cache(1, Arc::clone(&device))
            .write()
            .modify(0, |f: &mut FSInfo| *f = fsinfo);
        let fat = FATManager::new(bpb.fat1_offset(), Arc::clone(&device));
        let root_dir_cluster = bpb.root_cluster();
        // Set root next cluster
        fat.set_next_cluster(root_dir_cluster as u32, END_OF_CLUSTER);
        let mut name_bytes = [0x20u8; 11];
        name_bytes[0] = ROOT;
        let root_dir_entry = ShortDirEntry::new_from_name_bytes(
            root_dir_cluster as u32,
            &name_bytes,
            VirtFileType::Dir,
        );
        let fs = Arc::new(RwLock::new(Self {
            device,
            free_cluster_cnt: Arc::new(RwLock::new(free_cluster_cnt)),
            bpb,
            fat: Arc::new(RwLock::new(fat)),
            root_dir_entry: Arc::new(RwLock::new(root_dir_entry)),
        }));
        fs
    }
    pub fn open(device: Arc<dyn BlockDevice>) -> Arc<RwLock<Self>> {
        let bpb = get_block_cache(0, Arc::clone(&device))
            .read()
            .read(0, |bpb: &BIOSParameterBlock| *bpb);
        let free_cluster_cnt = get_block_cache(bpb.fat_info_sector(), Arc::clone(&device))
            .read()
            .read(0, |fsinfo: &FSInfo| {
                assert!(
                    fsinfo.check_signature(),
                    "Error loading fat32! Illegal signature"
                );
                fsinfo.free_cluster_cnt() as usize
            });
        let fat = FATManager::open(bpb.fat1_offset(), Arc::clone(&device));
        let root_dir_cluster = bpb.root_cluster();
        let mut name_bytes = [0x20u8; 11];
        name_bytes[0] = ROOT;
        let root_dir_entry = ShortDirEntry::new_from_name_bytes(
            root_dir_cluster as u32,
            &name_bytes,
            VirtFileType::Dir,
        );
        Arc::new(RwLock::new(Self {
            device,
            free_cluster_cnt: Arc::new(RwLock::new(free_cluster_cnt)),
            bpb,
            fat: Arc::new(RwLock::new(fat)),
            root_dir_entry: Arc::new(RwLock::new(root_dir_entry)),
        }))
    }
    fn clear_cluster(&self, cluster: u32) {
        let block_id = self.first_sector_of_cluster(cluster);
        for i in 0..self.sector_pre_cluster() {
            get_block_cache(block_id + i, Arc::clone(&self.device))
                .write()
                .modify(0, |cache: &mut [u8; BLOCK_SIZE]| {
                    cache.copy_from_slice(&[0u8; BLOCK_SIZE])
                })
        }
    }
    /// Generate a new cluster chain with length num in FAT table (not in struct ClusterChain), return the first cluster id
    pub fn alloc_cluster_chain(&self, num: usize, start_cluster: u32) -> Option<u32> {
        let free_cluster_cnt = self.free_cluster_cnt();
        if free_cluster_cnt < num {
            return None;
        }
        let first_cluster_id = self.fat.write().get_blank_cluster(start_cluster);
        assert!(first_cluster_id >= 2);
        self.clear_cluster(first_cluster_id);
        let mut curr_cluster_id = first_cluster_id;
        for _ in 1..num {
            let cluster_id = self.fat.write().get_blank_cluster(curr_cluster_id);
            assert!(cluster_id >= 2);
            self.clear_cluster(cluster_id);
            self.fat
                .write()
                .set_next_cluster(curr_cluster_id, cluster_id);
            curr_cluster_id = cluster_id;
        }
        self.fat
            .write()
            .set_next_cluster(curr_cluster_id, END_OF_CLUSTER);
        self.set_free_clusters(free_cluster_cnt - num);
        Some(first_cluster_id)
    }
    pub fn dealloc_cluster(&self, clusters: Vec<u32>) {
        let num = clusters.len();
        if num == 0 {
            return;
        }
        let free_cluster_cnt = self.free_cluster_cnt();
        for i in 0..num {
            self.fat.write().set_next_cluster(clusters[i], FREE_CLUSTER);
            self.fat.write().recycle(clusters[i]);
        }
        self.set_free_clusters(free_cluster_cnt + num);
    }
    pub fn root_dir_entry(&self) -> Arc<RwLock<ShortDirEntry>> {
        self.root_dir_entry.clone()
    }
}
