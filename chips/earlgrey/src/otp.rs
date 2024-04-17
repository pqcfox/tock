use kernel::utilities::StaticRef;
use lowrisc::registers::otp_ctrl_regs::OtpCtrlRegisters;
pub use lowrisc::otp::Otp;

use crate::registers::top_earlgrey::OTP_CTRL_CORE_BASE_ADDR;

pub const OTP_BASE: StaticRef<OtpCtrlRegisters> =
    unsafe { StaticRef::new(OTP_CTRL_CORE_BASE_ADDR as *const OtpCtrlRegisters) };
