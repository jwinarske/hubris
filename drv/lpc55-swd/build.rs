// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::Result;
use build_lpc55pins::PinConfig;
use serde::Deserialize;
use std::io::Write;

#[derive(Deserialize)]
struct TaskConfig {
    in_cfg: Vec<PinConfig>,
    out_cfg: Vec<PinConfig>,
    pins: Vec<PinConfig>,
    spi_num: usize,
}

fn generate_swd_functions(config: &TaskConfig) -> Result<()> {
    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = std::path::Path::new(&out_dir).join("swd.rs");
    let mut file = std::fs::File::create(&dest_path)?;

    let out_cfg = &config.out_cfg;
    let in_cfg = &config.in_cfg;
    let spi_periph = quote::format_ident!("Fc{}", config.spi_num);
    let flexcomm = quote::format_ident!("FLEXCOMM{}", config.spi_num);
    let spi_regs = quote::format_ident!("SPI{}", config.spi_num);

    // The RoT -> SP SWD control requires setting the IO functions at runtime
    // as opposed to just startup.
    writeln!(
        &mut file,
        "{}",
        quote::quote! {

        // io_out = MOSI on, MISO off
        fn switch_io_out(task : TaskId) {
            use drv_lpc55_gpio_api::*;
            let iocon = Pins::from(task);
            #(iocon.iocon_configure(#out_cfg).unwrap_lite();)*
        }
        // io_in = MOSI off, MISO on
        fn switch_io_in(task : TaskId) {
            use drv_lpc55_gpio_api::*;
            let iocon = Pins::from(task);
            #(iocon.iocon_configure(#in_cfg).unwrap_lite();)*
        }
        fn setup_spi(task : TaskId) -> spi_core::Spi {
            let syscon = Syscon::from(task);
                syscon.enable_clock(Peripheral::#spi_periph).unwrap_lite();
            syscon.leave_reset(Peripheral::#spi_periph).unwrap_lite();
            let flexcomm = unsafe { &*device::#flexcomm::ptr() };
            flexcomm.pselid.write(|w| w.persel().spi());
            let registers = unsafe { &*device::#spi_regs::ptr() };
            spi_core::Spi::from(registers)
        }
        }
    )?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    idol::server::build_server_support(
        "../../idl/sp-ctrl.idol",
        "server_stub.rs",
        idol::server::ServerStyle::InOrder,
    )?;

    let task_config = build_util::task_config::<TaskConfig>()?;

    generate_swd_functions(&task_config)?;
    build_lpc55pins::codegen(task_config.pins)?;

    Ok(())
}
