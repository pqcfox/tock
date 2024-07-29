use kernel::utilities::StaticRef;
pub use lowrisc::otp::Otp;
use lowrisc::registers::otp_ctrl_regs::OtpCtrlRegisters;

use crate::registers::top_earlgrey::OTP_CTRL_CORE_BASE_ADDR;

pub const OTP_BASE: StaticRef<OtpCtrlRegisters> =
    unsafe { StaticRef::new(OTP_CTRL_CORE_BASE_ADDR as *const OtpCtrlRegisters) };
