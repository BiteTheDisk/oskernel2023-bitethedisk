use core::panic::PanicInfo;

use crate::sbi::shutdown;

#[panic_handler]
fn _panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "[kernel] hart {} panicked at {}:{} {}",
            hartid!(),
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!(
            "[kernel] hart {} panicked: {}",
            hartid!(),
            info.message().unwrap()
        );
    }
    shutdown()
}
