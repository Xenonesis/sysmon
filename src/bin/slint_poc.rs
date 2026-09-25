slint::include_modules!();

use sysinfo::System;
use slint::Timer;
use std::time::Duration;
use std::rc::Rc;
use std::cell::RefCell;

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let mut sys = System::new_all();
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let sys_ref = Rc::new(RefCell::new(sys));

    let ui_handle = ui.as_weak();
    let timer = Timer::default();
    
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(1000), move || {
        let ui = ui_handle.unwrap();
        let mut sys = sys_ref.borrow_mut();
        
        sys.refresh_cpu_all();
        sys.refresh_memory();

        let global_cpu = sys.global_cpu_usage();
        let used_mem_mb = sys.used_memory() / 1024 / 1024;

        ui.set_cpu_usage(format!("{:.1}%", global_cpu).into());
        ui.set_ram_usage(format!("{} MB", used_mem_mb).into());
    });

    ui.run()
}
