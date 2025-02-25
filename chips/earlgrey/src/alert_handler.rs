// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header has to be included to be able to submit it to Tock
// It is up to ZeroRISC to decide if it keeps this header or not

use core::marker::PhantomData;

use kernel::{
    hil::opentitan_alerthandler::OpentTitanAlertHandlerClient,
    utilities::{
        cells::OptionalCell,
        registers::interfaces::{ReadWriteable, Readable, Writeable},
        StaticRef,
    },
};

use crate::registers::{
    alert_handler_regs::{
        AlertHandlerRegisters, ALERT_CAUSE, ALERT_CLASS_SHADOWED, ALERT_EN_SHADOWED,
        ALERT_HANDLER_PARAM_N_ALERTS, ALERT_HANDLER_PARAM_N_LOC_ALERT, ALERT_REGWEN,
        CLASSA_ACCUM_THRESH_SHADOWED, CLASSA_CLR_SHADOWED, CLASSA_CRASHDUMP_TRIGGER_SHADOWED,
        CLASSA_CTRL_SHADOWED, CLASSA_PHASE0_CYC_SHADOWED, CLASSA_PHASE1_CYC_SHADOWED,
        CLASSA_PHASE2_CYC_SHADOWED, CLASSA_PHASE3_CYC_SHADOWED, CLASSA_REGWEN, CLASSA_STATE,
        CLASSA_TIMEOUT_CYC_SHADOWED, CLASSB_ACCUM_THRESH_SHADOWED, CLASSB_CLR_SHADOWED,
        CLASSB_CRASHDUMP_TRIGGER_SHADOWED, CLASSB_CTRL_SHADOWED, CLASSB_PHASE0_CYC_SHADOWED,
        CLASSB_PHASE1_CYC_SHADOWED, CLASSB_PHASE2_CYC_SHADOWED, CLASSB_PHASE3_CYC_SHADOWED,
        CLASSB_REGWEN, CLASSB_STATE, CLASSB_TIMEOUT_CYC_SHADOWED, CLASSC_ACCUM_THRESH_SHADOWED,
        CLASSC_CLR_SHADOWED, CLASSC_CRASHDUMP_TRIGGER_SHADOWED, CLASSC_CTRL_SHADOWED,
        CLASSC_PHASE0_CYC_SHADOWED, CLASSC_PHASE1_CYC_SHADOWED, CLASSC_PHASE2_CYC_SHADOWED,
        CLASSC_PHASE3_CYC_SHADOWED, CLASSC_REGWEN, CLASSC_STATE, CLASSC_TIMEOUT_CYC_SHADOWED,
        CLASSD_ACCUM_THRESH_SHADOWED, CLASSD_CLR_SHADOWED, CLASSD_CRASHDUMP_TRIGGER_SHADOWED,
        CLASSD_CTRL_SHADOWED, CLASSD_PHASE0_CYC_SHADOWED, CLASSD_PHASE1_CYC_SHADOWED,
        CLASSD_PHASE2_CYC_SHADOWED, CLASSD_PHASE3_CYC_SHADOWED, CLASSD_REGWEN, CLASSD_STATE,
        CLASSD_TIMEOUT_CYC_SHADOWED, INTR, LOC_ALERT_CAUSE, LOC_ALERT_CLASS_SHADOWED,
        LOC_ALERT_EN_SHADOWED, LOC_ALERT_REGWEN, PING_TIMEOUT_CYC_SHADOWED, PING_TIMER_EN_SHADOWED,
        PING_TIMER_REGWEN,
    },
    top_earlgrey::AlertId,
};
pub(crate) const ALERTHANDLER_BASE: StaticRef<AlertHandlerRegisters> = unsafe {
    StaticRef::new(
        crate::registers::top_earlgrey::ALERT_HANDLER_BASE_ADDR as *const AlertHandlerRegisters,
    )
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocalAlertId {
    AlertPingFail = 0,
    EscalationPingFail = 1,
    AlertIntegFail = 2,
    EscalationIntegFail = 3,
    BusIntegrityFailure = 4,
    ShadowRegisterUpdateError = 5,
    ShadowRegisterStorageError = 6,
}

impl TryFrom<u32> for LocalAlertId {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::AlertPingFail),
            1 => Ok(Self::EscalationPingFail),
            2 => Ok(Self::AlertIntegFail),
            3 => Ok(Self::EscalationIntegFail),
            4 => Ok(Self::BusIntegrityFailure),
            5 => Ok(Self::ShadowRegisterUpdateError),
            6 => Ok(Self::ShadowRegisterStorageError),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum AlertClass {
    ClassA = 0,
    ClassB = 1,
    ClassC = 2,
    ClassD = 3,
}

impl From<ALERT_CLASS_SHADOWED::CLASS_A_0::Value> for AlertClass {
    fn from(value: ALERT_CLASS_SHADOWED::CLASS_A_0::Value) -> Self {
        match value {
            ALERT_CLASS_SHADOWED::CLASS_A_0::Value::CLASSA => Self::ClassA,
            ALERT_CLASS_SHADOWED::CLASS_A_0::Value::CLASSB => Self::ClassB,
            ALERT_CLASS_SHADOWED::CLASS_A_0::Value::CLASSC => Self::ClassC,
            ALERT_CLASS_SHADOWED::CLASS_A_0::Value::CLASSD => Self::ClassD,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum AlertState {
    Idle = 0x0,
    Timeout = 0x1,
    FsmError = 0x2,
    Terminal = 0x3,
    Phase0 = 0x4,
    Phase1 = 0x5,
    Phase2 = 0x6,
    Phase3 = 0x7,
}

macro_rules! into_alert_state {
    {$class_state:ident} => {
        impl From<$class_state::$class_state::Value> for AlertState {
            fn from(value: $class_state::$class_state::Value) -> Self {
                match value {
                    $class_state::$class_state::Value::IDLE => AlertState::Idle,
                    $class_state::$class_state::Value::TIMEOUT => AlertState::Timeout,
                    $class_state::$class_state::Value::FSMERROR => AlertState::FsmError,
                    $class_state::$class_state::Value::TERMINAL => AlertState::Terminal,
                    $class_state::$class_state::Value::PHASE0 => AlertState::Phase0,
                    $class_state::$class_state::Value::PHASE1 => AlertState::Phase1,
                    $class_state::$class_state::Value::PHASE2 => AlertState::Phase2,
                    $class_state::$class_state::Value::PHASE3 => AlertState::Phase3,
                }
            }
        }
    }
}
into_alert_state! {CLASSA_STATE}
into_alert_state! {CLASSB_STATE}
into_alert_state! {CLASSC_STATE}
into_alert_state! {CLASSD_STATE}

#[derive(Clone, Copy, Debug)]
pub enum AlertPhaseSignalOutput {
    Phase0 = 0,
    Phase1 = 1,
    Phase2 = 2,
    Phase3 = 3,
}

pub struct AlertClassConfiguration {
    accumulation_threshold: u32,
    timeout: Option<u32>,
    phase0_length: u32,
    phase1_length: u32,
    phase2_length: u32,
    phase3_length: u32,
    signal0_phase: Option<AlertPhaseSignalOutput>,
    signal1_phase: Option<AlertPhaseSignalOutput>,
    signal2_phase: Option<AlertPhaseSignalOutput>,
    signal3_phase: Option<AlertPhaseSignalOutput>,
    crashdump_phase: AlertPhaseSignalOutput,
    lock_escalation_counter: bool,
}

const ALERTFLAGS_NUMBER_OF_LOCAL_ALERTS: usize = ALERT_HANDLER_PARAM_N_LOC_ALERT as usize;
const ALERTFLAGS_NUMBER_OF_ALERTS: usize = ALERT_HANDLER_PARAM_N_ALERTS as usize;

/// Small bitfield implmentation that stores `BITS` number of flags in `WORDS` number of u32s. Condition: 32*`WORDS`>=`BITS`. Flags can only be set or checked if they are set (`is_set`), they can't be cleared as the role of this struct is to keep track of flags that have been handled
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AlertBitfield<const WORDS: usize, const BITS: usize, AlertType: TryFrom<u32>> {
    alerts: [u32; WORDS],
    _phantom: PhantomData<AlertType>,
}

// type alias for alert flags from all peripherals
pub type AlertFlags = AlertBitfield<3, ALERTFLAGS_NUMBER_OF_ALERTS, AlertId>;
// type alias for alert flags from AlertHandler peripheral, called local alert flags
pub type LocalAlertFlags = AlertBitfield<1, ALERTFLAGS_NUMBER_OF_LOCAL_ALERTS, LocalAlertId>;

impl<const SIZE: usize, const BITS: usize, AlertType: TryFrom<u32>>
    AlertBitfield<SIZE, BITS, AlertType>
{
    pub const fn empty() -> Self {
        Self {
            alerts: [0; SIZE],
            _phantom: PhantomData,
        }
    }

    /// Iterate through each flag that is set in `current_flags` and is also not set in `Self`, call `f(..)` on them and mark them as set in `Self. Return true if at least one flag was handled
    /// # Example:
    /// ```ignore
    ///                 0123
    ///         Self :  TTFF
    /// current_flags:  TFTF
    ///        f(...):  __x_
    /// Self (output):  TTTF
    /// ```
    /// `f` will be called only on flag no. 2 as it appears set in `current_flags` but no in `Self`, this flag will bet appear set in `Self` and the function will return true as at least one flag was handled
    pub fn for_each_new<F>(
        &mut self,
        current_flags: &AlertBitfield<SIZE, BITS, AlertType>,
        mut f: F,
    ) -> bool
    where
        F: FnMut(AlertType),
    {
        let mut at_least_one_new = false;
        for flag_index in 0..BITS {
            if !self.is_set(flag_index) & current_flags.is_set(flag_index) {
                // flag `i` was not previously handled and now is raised
                let id = AlertType::try_from(flag_index as u32);
                // manually unwrap the Option< Id >, otherwise further constraints should have been added on the `AlertType` generic

                id.map_or_else(
                    |_| panic!("Invalid id = {:?} found", flag_index),
                    |id| {
                        // handle the flag
                        f(id);
                        self.set(flag_index);
                        at_least_one_new = true;
                    },
                )
            }
        }
        at_least_one_new
    }

    /// set flag numbered `id`. If the id doesn't exist, no flag is set.
    pub fn set(&mut self, id: usize) {
        if id <= BITS {
            // split the id into a `word_no` index that determines in which 32bit word the flags is present and an `index` that determines the bit number inside the 32bit word. build a mask with the correct bit number set
            // 31......5 43210
            // [word_no][index]
            let word_no = id >> 5;
            let index = id & 0x1F;
            let mask = 1 << index;
            self.alerts[word_no] |= mask;
        }
    }

    /// check if the flag with the given id is set. For flags with invalid ids the funciton returns false
    fn is_set(&self, id: usize) -> bool {
        if id <= BITS {
            // split the id into a `word_no` index that determines in which 32bit word the flags is present and an `index` that determines the bit number inside the 32bit word. build a mask with the correct bit number set
            // 31......5 43210
            // [word_no][index]
            let word_no = id >> 5;
            let index = id & 0x1F;
            let mask = 1 << index;
            (self.alerts[word_no] & mask) > 0
        } else {
            false
        }
    }
}

pub struct AlertHandler {
    registers: StaticRef<AlertHandlerRegisters>,
    capsule_ref: OptionalCell<&'static dyn OpentTitanAlertHandlerClient>,
}

// Macro that avoids repeating class configuration logic for alert classes A-D.
macro_rules! configure_class {
    {
        function = $function:ident,
        class = $class:expr,
        ctrl_shadowed = $ctrl_shadowed:ident,
        accum_thresh_shadowed = $accum_thresh_shadowed:ident,
        timeout_cyc_shadowed = $timeout_cyc_shadowed:ident,
        phase0_cyc_shadowed = $phase0_cyc_shadowed:ident,
        phase1_cyc_shadowed = $phase1_cyc_shadowed:ident,
        phase2_cyc_shadowed = $phase2_cyc_shadowed:ident,
        phase3_cyc_shadowed = $phase3_cyc_shadowed:ident,
        crashdump_trigger_shadowed = $crashdump_trigger_shadowed:ident,
        ACCUM_THRESH_SHADOWED = $ACCUM_THRESH_SHADOWED:ident,
        TIMEOUT_CYC_SHADOWED = $TIMEOUT_CYC_SHADOWED:ident,
        PHASE0_CYC_SHADOWED = $PHASE0_CYC_SHADOWED:ident,
        PHASE1_CYC_SHADOWED = $PHASE1_CYC_SHADOWED:ident,
        PHASE2_CYC_SHADOWED = $PHASE2_CYC_SHADOWED:ident,
        PHASE3_CYC_SHADOWED = $PHASE3_CYC_SHADOWED:ident,
        CTRL_SHADOWED = $CTRL_SHADOWED:ident,
        CRASHDUMP_TRIGGER_SHADOWED = $CRASHDUMP_TRIGGER_SHADOWED:ident,
    } => {
        fn $function(
            &self,
            config: Option<AlertClassConfiguration>,
            lock: bool,
        ) -> Result<(), ()> {
            if self.is_class_locked($class) {
                return Err(());
            }

            let class_ctrl_reg = &self.registers.$ctrl_shadowed;

            if let Some(configuration) = config {
                // configure accumulation threshold for this class of alerts
                let class_accumulation_threshold_reg = &self.registers.$accum_thresh_shadowed;
                class_accumulation_threshold_reg.write(
                    $ACCUM_THRESH_SHADOWED::$ACCUM_THRESH_SHADOWED
                        .val(configuration.accumulation_threshold),
                );

                class_accumulation_threshold_reg.write(
                    $ACCUM_THRESH_SHADOWED::$ACCUM_THRESH_SHADOWED
                        .val(configuration.accumulation_threshold),
                );

                // configure timeout for this class of alerts
                let class_timeout_reg = &self.registers.$timeout_cyc_shadowed;

                // a timeout of None signifies that no timeout is needed for this class of alerts (0 in register)
                class_timeout_reg.write(
                    $TIMEOUT_CYC_SHADOWED::$TIMEOUT_CYC_SHADOWED
                        .val(configuration.timeout.unwrap_or(0)),
                );
                class_timeout_reg.write(
                    $TIMEOUT_CYC_SHADOWED::$TIMEOUT_CYC_SHADOWED
                        .val(configuration.timeout.unwrap_or(0)),
                );

                let class_phase0_reg = &self.registers.$phase0_cyc_shadowed;
                let class_phase1_reg = &self.registers.$phase1_cyc_shadowed;
                let class_phase2_reg = &self.registers.$phase2_cyc_shadowed;
                let class_phase3_reg = &self.registers.$phase3_cyc_shadowed;

                class_phase0_reg.write(
                    $PHASE0_CYC_SHADOWED::$PHASE0_CYC_SHADOWED
                        .val(configuration.phase0_length),
                );
                class_phase0_reg.write(
                    $PHASE0_CYC_SHADOWED::$PHASE0_CYC_SHADOWED
                        .val(configuration.phase0_length),
                );
                class_phase1_reg.write(
                    $PHASE1_CYC_SHADOWED::$PHASE1_CYC_SHADOWED
                        .val(configuration.phase1_length),
                );
                class_phase1_reg.write(
                    $PHASE1_CYC_SHADOWED::$PHASE1_CYC_SHADOWED
                        .val(configuration.phase1_length),
                );
                class_phase2_reg.write(
                    $PHASE2_CYC_SHADOWED::$PHASE2_CYC_SHADOWED
                        .val(configuration.phase2_length),
                );
                class_phase2_reg.write(
                    $PHASE2_CYC_SHADOWED::$PHASE2_CYC_SHADOWED
                        .val(configuration.phase2_length),
                );
                class_phase3_reg.write(
                    $PHASE3_CYC_SHADOWED::$PHASE3_CYC_SHADOWED
                        .val(configuration.phase3_length),
                );
                class_phase3_reg.write(
                    $PHASE3_CYC_SHADOWED::$PHASE3_CYC_SHADOWED
                        .val(configuration.phase3_length),
                );

                let (signal0_en, signal0_phase) = configuration
                    .signal0_phase
                    .map_or_else(|| (0u32, 0u32), |x| (1u32, x as u32));
                let (signal1_en, signal1_phase) = configuration
                    .signal1_phase
                    .map_or_else(|| (0u32, 0u32), |x| (1u32, x as u32));
                let (signal2_en, signal2_phase) = configuration
                    .signal2_phase
                    .map_or_else(|| (0u32, 0u32), |x| (1u32, x as u32));
                let (signal3_en, signal3_phase) = configuration
                    .signal3_phase
                    .map_or_else(|| (0u32, 0u32), |x| (1u32, x as u32));

                let class_ctrl_value = $CTRL_SHADOWED::EN.val(1)
                    + $CTRL_SHADOWED::EN_E0.val(signal0_en)
                    + $CTRL_SHADOWED::MAP_E0.val(signal0_phase)
                    + $CTRL_SHADOWED::EN_E1.val(signal1_en)
                    + $CTRL_SHADOWED::MAP_E1.val(signal1_phase)
                    + $CTRL_SHADOWED::EN_E2.val(signal2_en)
                    + $CTRL_SHADOWED::MAP_E2.val(signal2_phase)
                    + $CTRL_SHADOWED::EN_E3.val(signal3_en)
                    + $CTRL_SHADOWED::MAP_E3.val(signal3_phase)
                    + $CTRL_SHADOWED::LOCK.val(configuration.lock_escalation_counter as u32);

                class_ctrl_reg.write(class_ctrl_value);
                class_ctrl_reg.write(class_ctrl_value);

                let class_crashdump_trigger = &self.registers.$crashdump_trigger_shadowed;

                class_crashdump_trigger.write(
                    $CRASHDUMP_TRIGGER_SHADOWED::$CRASHDUMP_TRIGGER_SHADOWED
                        .val(configuration.crashdump_phase as u32),
                );
                class_crashdump_trigger.write(
                    $CRASHDUMP_TRIGGER_SHADOWED::$CRASHDUMP_TRIGGER_SHADOWED
                        .val(configuration.crashdump_phase as u32),
                );
            } else {
                // disable escalation method for this class
                // read, modify_no_read, modify_no_read is needed in order to modify a single bit in a shadowed register
                let a = class_ctrl_reg.extract();
                class_ctrl_reg.modify_no_read(a, $CTRL_SHADOWED::EN.val(0));
                class_ctrl_reg.modify_no_read(a, $CTRL_SHADOWED::EN.val(0));
            }
            if lock {
                self.lock_register_writeprotect_class($class);
            }
            Ok(())
        }

    }
}
impl AlertHandler {
    pub fn new() -> Self {
        Self {
            registers: ALERTHANDLER_BASE,
            capsule_ref: OptionalCell::empty(),
        }
    }

    pub fn set_client(&self, client: &'static dyn OpentTitanAlertHandlerClient) {
        self.capsule_ref.set(client);
    }

    /* ALERT */

    /// enable alert by `alertid` and configure peripheral to classify it by `class`. If `lock` is true, the configuration registers will be locked until a chip restart.
    pub fn configure_alert(
        &self,
        alertid: AlertId,
        class: AlertClass,
        lock: bool,
    ) -> Result<(), ()> {
        let alert_num = alertid as u32;

        if self.is_alert_locked(alertid) {
            return Err(());
        }

        // enable alert
        self.registers.alert_en_shadowed[alert_num as usize]
            .write(ALERT_EN_SHADOWED::EN_A_0.val(1));
        self.registers.alert_en_shadowed[alert_num as usize]
            .write(ALERT_EN_SHADOWED::EN_A_0.val(1));

        // configure alert class
        let class_fieldvalue = match class {
            AlertClass::ClassA => ALERT_CLASS_SHADOWED::CLASS_A_0::CLASSA,
            AlertClass::ClassB => ALERT_CLASS_SHADOWED::CLASS_A_0::CLASSB,
            AlertClass::ClassC => ALERT_CLASS_SHADOWED::CLASS_A_0::CLASSC,
            AlertClass::ClassD => ALERT_CLASS_SHADOWED::CLASS_A_0::CLASSD,
        };
        self.registers.alert_class_shadowed[alert_num as usize].write(class_fieldvalue);
        self.registers.alert_class_shadowed[alert_num as usize].write(class_fieldvalue);

        // lock register write protection
        if lock {
            self.registers.alert_regwen[alert_num as usize].write(ALERT_REGWEN::EN_0.val(0));
        }
        Ok(())
    }

    /// try to disable the alert with `alertid`, will fail if alert configuration is locked. Alerts should be disabled only if they are known to be faulty
    pub fn disable_alert(&self, alertid: AlertId) -> Result<(), ()> {
        let alert_num = alertid as u32;

        if self.is_alert_locked(alertid) {
            return Err(());
        }
        // disable alert
        self.registers.alert_en_shadowed[alert_num as usize]
            .write(ALERT_EN_SHADOWED::EN_A_0.val(1));
        self.registers.alert_en_shadowed[alert_num as usize]
            .write(ALERT_EN_SHADOWED::EN_A_0.val(1));

        Ok(())
    }

    pub fn is_alert_cause_set(&self, cause: AlertId) -> bool {
        self.registers.alert_cause[cause as usize].is_set(ALERT_CAUSE::A_0)
    }

    // try to clear the alert flag, if the alert reason is still present the flag will remain set
    pub fn clear_alert_cause(&self, cause: AlertId) {
        self.registers.alert_cause[cause as usize].write(ALERT_CAUSE::A_0.val(1));
    }

    pub fn snapshot_alert_causes(&self) -> AlertFlags {
        let mut alert_flags = AlertFlags::empty();
        for i in 0..ALERT_HANDLER_PARAM_N_ALERTS as usize {
            if self.registers.alert_cause[i].is_set(ALERT_CAUSE::A_0) {
                alert_flags.set(i);
            }
        }
        alert_flags
    }

    pub fn is_alert_enabled(&self, alertid: AlertId) -> bool {
        self.registers.alert_en_shadowed[alertid as usize].is_set(ALERT_EN_SHADOWED::EN_A_0)
    }

    pub fn is_alert_locked(&self, alertid: AlertId) -> bool {
        !self.registers.alert_regwen[alertid as usize].is_set(ALERT_REGWEN::EN_0)
    }

    /* LOCAL ALERT */

    /// enable local alert by `localid` and configure peripheral to classify it by `class`. If `lock` is true, the configuration registers will be locked until a chip restart. If configuration is already locked this function will return Err(()).
    pub fn configure_local_alert(
        &self,
        localid: LocalAlertId,
        class: AlertClass,
        locked: bool,
    ) -> Result<(), ()> {
        let alert_num = localid as u32;

        if self.is_local_alert_locked(localid) {
            return Err(());
        }
        // unlock register write protection
        self.registers.loc_alert_regwen[alert_num as usize].write(LOC_ALERT_REGWEN::EN_0.val(1));

        // enable alert
        self.registers.loc_alert_en_shadowed[alert_num as usize]
            .write(LOC_ALERT_EN_SHADOWED::EN_LA_0.val(1));
        self.registers.loc_alert_en_shadowed[alert_num as usize]
            .write(LOC_ALERT_EN_SHADOWED::EN_LA_0.val(1));

        // configure alert class
        let class_fieldvalue = match class {
            AlertClass::ClassA => LOC_ALERT_CLASS_SHADOWED::CLASS_LA_0::CLASSA,
            AlertClass::ClassB => LOC_ALERT_CLASS_SHADOWED::CLASS_LA_0::CLASSB,
            AlertClass::ClassC => LOC_ALERT_CLASS_SHADOWED::CLASS_LA_0::CLASSC,
            AlertClass::ClassD => LOC_ALERT_CLASS_SHADOWED::CLASS_LA_0::CLASSD,
        };
        self.registers.loc_alert_class_shadowed[alert_num as usize].write(class_fieldvalue);
        self.registers.loc_alert_class_shadowed[alert_num as usize].write(class_fieldvalue);

        // lock register write protection
        if locked {
            self.registers.loc_alert_regwen[alert_num as usize]
                .write(LOC_ALERT_REGWEN::EN_0.val(0));
        }
        Ok(())
    }

    pub fn is_local_alert_cause_set(&self, cause: LocalAlertId) -> bool {
        self.registers.loc_alert_cause[cause as usize].is_set(LOC_ALERT_CAUSE::LA_0)
    }

    pub fn is_local_alert_locked(&self, localid: LocalAlertId) -> bool {
        !self.registers.loc_alert_regwen[localid as usize].is_set(LOC_ALERT_REGWEN::EN_0)
    }

    pub fn snapshot_local_alert_causes(&self) -> LocalAlertFlags {
        let mut alert_flags = LocalAlertFlags::empty();
        for i in 0..ALERTFLAGS_NUMBER_OF_LOCAL_ALERTS {
            if self.registers.loc_alert_cause[i].is_set(LOC_ALERT_CAUSE::LA_0) {
                alert_flags.set(i);
            }
        }
        alert_flags
    }

    // try to clear the alert flag, if the alert reason is still present the flag will remain set
    pub fn clear_local_alert_cause(&self, cause: LocalAlertId) {
        self.registers.loc_alert_cause[cause as usize].write(LOC_ALERT_CAUSE::LA_0.val(1));
    }

    /* CLASSES */
    configure_class! {
        function = configure_classa,
        class = AlertClass::ClassA,
        ctrl_shadowed = classa_ctrl_shadowed,
        accum_thresh_shadowed = classa_accum_thresh_shadowed,
        timeout_cyc_shadowed = classa_timeout_cyc_shadowed,
        phase0_cyc_shadowed = classa_phase0_cyc_shadowed,
        phase1_cyc_shadowed = classa_phase1_cyc_shadowed,
        phase2_cyc_shadowed = classa_phase2_cyc_shadowed,
        phase3_cyc_shadowed = classa_phase3_cyc_shadowed,
        crashdump_trigger_shadowed = classa_crashdump_trigger_shadowed,
        ACCUM_THRESH_SHADOWED = CLASSA_ACCUM_THRESH_SHADOWED,
        TIMEOUT_CYC_SHADOWED = CLASSA_TIMEOUT_CYC_SHADOWED,
        PHASE0_CYC_SHADOWED = CLASSA_PHASE0_CYC_SHADOWED,
        PHASE1_CYC_SHADOWED = CLASSA_PHASE1_CYC_SHADOWED,
        PHASE2_CYC_SHADOWED = CLASSA_PHASE2_CYC_SHADOWED,
        PHASE3_CYC_SHADOWED = CLASSA_PHASE3_CYC_SHADOWED,
        CTRL_SHADOWED = CLASSA_CTRL_SHADOWED,
        CRASHDUMP_TRIGGER_SHADOWED = CLASSA_CRASHDUMP_TRIGGER_SHADOWED,
    }
    configure_class! {
        function = configure_classb,
        class = AlertClass::ClassB,
        ctrl_shadowed = classb_ctrl_shadowed,
        accum_thresh_shadowed = classb_accum_thresh_shadowed,
        timeout_cyc_shadowed = classb_timeout_cyc_shadowed,
        phase0_cyc_shadowed = classb_phase0_cyc_shadowed,
        phase1_cyc_shadowed = classb_phase1_cyc_shadowed,
        phase2_cyc_shadowed = classb_phase2_cyc_shadowed,
        phase3_cyc_shadowed = classb_phase3_cyc_shadowed,
        crashdump_trigger_shadowed = classb_crashdump_trigger_shadowed,
        ACCUM_THRESH_SHADOWED = CLASSB_ACCUM_THRESH_SHADOWED,
        TIMEOUT_CYC_SHADOWED = CLASSB_TIMEOUT_CYC_SHADOWED,
        PHASE0_CYC_SHADOWED = CLASSB_PHASE0_CYC_SHADOWED,
        PHASE1_CYC_SHADOWED = CLASSB_PHASE1_CYC_SHADOWED,
        PHASE2_CYC_SHADOWED = CLASSB_PHASE2_CYC_SHADOWED,
        PHASE3_CYC_SHADOWED = CLASSB_PHASE3_CYC_SHADOWED,
        CTRL_SHADOWED = CLASSB_CTRL_SHADOWED,
        CRASHDUMP_TRIGGER_SHADOWED = CLASSB_CRASHDUMP_TRIGGER_SHADOWED,
    }
    configure_class! {
        function = configure_classc,
        class = AlertClass::ClassC,
        ctrl_shadowed = classc_ctrl_shadowed,
        accum_thresh_shadowed = classc_accum_thresh_shadowed,
        timeout_cyc_shadowed = classc_timeout_cyc_shadowed,
        phase0_cyc_shadowed = classc_phase0_cyc_shadowed,
        phase1_cyc_shadowed = classc_phase1_cyc_shadowed,
        phase2_cyc_shadowed = classc_phase2_cyc_shadowed,
        phase3_cyc_shadowed = classc_phase3_cyc_shadowed,
        crashdump_trigger_shadowed = classc_crashdump_trigger_shadowed,
        ACCUM_THRESH_SHADOWED = CLASSC_ACCUM_THRESH_SHADOWED,
        TIMEOUT_CYC_SHADOWED = CLASSC_TIMEOUT_CYC_SHADOWED,
        PHASE0_CYC_SHADOWED = CLASSC_PHASE0_CYC_SHADOWED,
        PHASE1_CYC_SHADOWED = CLASSC_PHASE1_CYC_SHADOWED,
        PHASE2_CYC_SHADOWED = CLASSC_PHASE2_CYC_SHADOWED,
        PHASE3_CYC_SHADOWED = CLASSC_PHASE3_CYC_SHADOWED,
        CTRL_SHADOWED = CLASSC_CTRL_SHADOWED,
        CRASHDUMP_TRIGGER_SHADOWED = CLASSC_CRASHDUMP_TRIGGER_SHADOWED,
    }
    configure_class! {
        function = configure_classd,
        class = AlertClass::ClassD,
        ctrl_shadowed = classd_ctrl_shadowed,
        accum_thresh_shadowed = classd_accum_thresh_shadowed,
        timeout_cyc_shadowed = classd_timeout_cyc_shadowed,
        phase0_cyc_shadowed = classd_phase0_cyc_shadowed,
        phase1_cyc_shadowed = classd_phase1_cyc_shadowed,
        phase2_cyc_shadowed = classd_phase2_cyc_shadowed,
        phase3_cyc_shadowed = classd_phase3_cyc_shadowed,
        crashdump_trigger_shadowed = classd_crashdump_trigger_shadowed,
        ACCUM_THRESH_SHADOWED = CLASSD_ACCUM_THRESH_SHADOWED,
        TIMEOUT_CYC_SHADOWED = CLASSD_TIMEOUT_CYC_SHADOWED,
        PHASE0_CYC_SHADOWED = CLASSD_PHASE0_CYC_SHADOWED,
        PHASE1_CYC_SHADOWED = CLASSD_PHASE1_CYC_SHADOWED,
        PHASE2_CYC_SHADOWED = CLASSD_PHASE2_CYC_SHADOWED,
        PHASE3_CYC_SHADOWED = CLASSD_PHASE3_CYC_SHADOWED,
        CTRL_SHADOWED = CLASSD_CTRL_SHADOWED,
        CRASHDUMP_TRIGGER_SHADOWED = CLASSD_CRASHDUMP_TRIGGER_SHADOWED,
    }

    /// try to configure how alerts of certain class (`class`) should behave during escalation.
    /// * `class` - the class that should be configured
    /// * `lock` - should the configuration be locked (until a chip reset)
    /// * `config` - None - alerts of this class should not trigger escalation
    /// * `config` - Some(): -  alerts of this class should trigger escalation and escalation should behave according to these parameters:
    ///     - accumulation_threshold - (CLASSx_ACCUM_THRESH_SHADOWED) once this many alerts of this class are triggered the escalation should start
    ///     - timeout - (CLASSx_TIMEOUT_CYC_SHADOWED) if the interrupt corresponding to this class is not handled within the specified amount of cycles, escalation will be triggered. can be deactivated by setting this parameter to None
    ///     - phase{0,1,2,3}_length - (CLASSx_PHASEx_CYC_SHADOWED) duration of escalation phase X for this class
    ///     - signal{0,1,2,3}_phase - (CLASSx_CTRL_SHADOWED::{MAP_ENx,ENx})  controls during which phase should signal X be triggered. Signal can be configured to not trigger by setting this paramter to None
    ///     - crashdump_phase - (CLASSx_CRASHDUMP_TRIGGER_SHADOWED) controls during which phase should the crashdump information be recorded
    ///     - lock_escalation_counter - if this is true, there is no way to stop the escalation protocol for this class once it has been triggered
    pub fn configure_class(
        &self,
        class: AlertClass,
        config: Option<AlertClassConfiguration>,
        lock: bool,
    ) -> Result<(), ()> {
        match class {
            AlertClass::ClassA => self.configure_classa(config, lock),
            AlertClass::ClassB => self.configure_classb(config, lock),
            AlertClass::ClassC => self.configure_classc(config, lock),
            AlertClass::ClassD => self.configure_classd(config, lock),
        }
    }

    fn is_class_locked(&self, class: AlertClass) -> bool {
        match class {
            AlertClass::ClassA => !&self
                .registers
                .classa_regwen
                .is_set(CLASSA_REGWEN::CLASSA_REGWEN),
            AlertClass::ClassB => !&self
                .registers
                .classb_regwen
                .is_set(CLASSB_REGWEN::CLASSB_REGWEN),
            AlertClass::ClassC => !&self
                .registers
                .classc_regwen
                .is_set(CLASSC_REGWEN::CLASSC_REGWEN),
            AlertClass::ClassD => !&self
                .registers
                .classd_regwen
                .is_set(CLASSD_REGWEN::CLASSD_REGWEN),
        }
    }

    fn lock_register_writeprotect_class(&self, class: AlertClass) {
        match class {
            AlertClass::ClassA => self
                .registers
                .classa_regwen
                .write(CLASSA_REGWEN::CLASSA_REGWEN.val(0)),
            AlertClass::ClassB => self
                .registers
                .classb_regwen
                .write(CLASSB_REGWEN::CLASSB_REGWEN.val(0)),
            AlertClass::ClassC => self
                .registers
                .classc_regwen
                .write(CLASSC_REGWEN::CLASSC_REGWEN.val(0)),
            AlertClass::ClassD => self
                .registers
                .classd_regwen
                .write(CLASSD_REGWEN::CLASSD_REGWEN.val(0)),
        }
    }

    pub fn class_state(&self, class: AlertClass) -> AlertState {
        match class {
            AlertClass::ClassA => {
                // PANIC: all (8) variants of CLASSA_STATE register (3 bits) are
                // encoded in a state in no case can the unwrap fail.
                self.registers
                    .classa_state
                    .read_as_enum::<CLASSA_STATE::CLASSA_STATE::Value>(CLASSA_STATE::CLASSA_STATE)
                    .unwrap()
                    .into()
            }
            AlertClass::ClassB => {
                // PANIC: all (8) variants of CLASSB_STATE register (3 bits) are
                // encoded in a state in no case can the unwrap fail.
                self.registers
                    .classb_state
                    .read_as_enum::<CLASSB_STATE::CLASSB_STATE::Value>(CLASSB_STATE::CLASSB_STATE)
                    .unwrap()
                    .into()
            }
            AlertClass::ClassC => {
                // PANIC: all (8) variants of CLASSC_STATE register (3 bits) are
                // encoded in a state in no case can the unwrap fail.
                self.registers
                    .classc_state
                    .read_as_enum::<CLASSC_STATE::CLASSC_STATE::Value>(CLASSC_STATE::CLASSC_STATE)
                    .unwrap()
                    .into()
            }
            AlertClass::ClassD => {
                // PANIC: all (8) variants of CLASSD_STATE register (3 bits) are
                // encoded in a state in no case can the unwrap fail.
                self.registers
                    .classd_state
                    .read_as_enum::<CLASSD_STATE::CLASSD_STATE::Value>(CLASSD_STATE::CLASSD_STATE)
                    .unwrap()
                    .into()
            }
        }
    }

    pub fn clear_esclation(&self, class: AlertClass) {
        // Write twice because registers are shadowed.
        match class {
            AlertClass::ClassA => {
                let reg = &self.registers.classa_clr_shadowed;
                reg.write(CLASSA_CLR_SHADOWED::CLASSA_CLR_SHADOWED.val(1));
                reg.write(CLASSA_CLR_SHADOWED::CLASSA_CLR_SHADOWED.val(1));
            }
            AlertClass::ClassB => {
                let reg = &self.registers.classb_clr_shadowed;
                reg.write(CLASSB_CLR_SHADOWED::CLASSB_CLR_SHADOWED.val(1));
                reg.write(CLASSB_CLR_SHADOWED::CLASSB_CLR_SHADOWED.val(1));
            }
            AlertClass::ClassC => {
                let reg = &self.registers.classc_clr_shadowed;
                reg.write(CLASSC_CLR_SHADOWED::CLASSC_CLR_SHADOWED.val(1));
                reg.write(CLASSC_CLR_SHADOWED::CLASSC_CLR_SHADOWED.val(1));
            }
            AlertClass::ClassD => {
                let reg = &self.registers.classd_clr_shadowed;
                reg.write(CLASSD_CLR_SHADOWED::CLASSD_CLR_SHADOWED.val(1));
                reg.write(CLASSD_CLR_SHADOWED::CLASSD_CLR_SHADOWED.val(1));
            }
        }
    }

    /* PING TIMER */

    pub fn enable_ping_timer(&self, cycles: u16, lock: bool) -> Result<(), ()> {
        // check if register write protection is locked
        if !self
            .registers
            .ping_timer_regwen
            .is_set(PING_TIMER_REGWEN::PING_TIMER_REGWEN)
        {
            return Err(());
        }

        // confiugre timeout
        self.registers
            .ping_timeout_cyc_shadowed
            .write(PING_TIMEOUT_CYC_SHADOWED::PING_TIMEOUT_CYC_SHADOWED.val(cycles as u32));
        self.registers
            .ping_timeout_cyc_shadowed
            .write(PING_TIMEOUT_CYC_SHADOWED::PING_TIMEOUT_CYC_SHADOWED.val(cycles as u32));

        // enable the ping timer
        self.registers
            .ping_timer_en_shadowed
            .write(PING_TIMER_EN_SHADOWED::PING_TIMER_EN_SHADOWED.val(1));
        self.registers
            .ping_timer_en_shadowed
            .write(PING_TIMER_EN_SHADOWED::PING_TIMER_EN_SHADOWED.val(1));

        // lock register write protection
        if lock {
            self.registers
                .ping_timer_regwen
                .write(PING_TIMER_REGWEN::PING_TIMER_REGWEN.val(0));
        }
        Ok(())
    }

    /* INTERRUPTS */

    pub fn enable_interrupt(&self, class: AlertClass) {
        // Write 1 in INTR_ENABLE to enable an interrupt.
        self.registers.intr_enable.modify(match class {
            AlertClass::ClassA => INTR::CLASSA.val(1),
            AlertClass::ClassB => INTR::CLASSB.val(1),
            AlertClass::ClassC => INTR::CLASSC.val(1),
            AlertClass::ClassD => INTR::CLASSD.val(1),
        })
    }

    pub fn enable_all_interrupts(&self) {
        self.enable_interrupt(AlertClass::ClassA);
        self.enable_interrupt(AlertClass::ClassB);
        self.enable_interrupt(AlertClass::ClassC);
        self.enable_interrupt(AlertClass::ClassD);
    }

    pub fn clear_interrupt(&self, class: AlertClass) {
        // Write 1 to INTR_STATE to clear the interrupt.
        self.registers.intr_state.modify(match class {
            AlertClass::ClassA => INTR::CLASSA.val(1),
            AlertClass::ClassB => INTR::CLASSB.val(1),
            AlertClass::ClassC => INTR::CLASSC.val(1),
            AlertClass::ClassD => INTR::CLASSD.val(1),
        })
    }

    pub fn handle_interrupt(&self, class: AlertClass) {
        // TODO: implement this
        match class {
            AlertClass::ClassA => {}
            AlertClass::ClassB => {}
            AlertClass::ClassC => {}
            AlertClass::ClassD => {}
        }
    }

    /* ALERT HANDLING and ALERT TESTING */

    /// test alert handling by generating a ShadowRegisterUpdateError
    pub fn fail_shadow_reg(&self) {
        let _ = self.enable_ping_timer(0x1000, false);
        self.registers
            .ping_timer_regwen
            .write(PING_TIMER_REGWEN::PING_TIMER_REGWEN.val(1));

        self.registers.ping_timeout_cyc_shadowed.set(0x0ABCD);
        self.registers.ping_timeout_cyc_shadowed.set(0x43210);
    }

    /// function called when a local alert happened. Should return `AlertWasHandled::Yes` if the source of the alert was handled and the caller should clear the alert flag
    pub fn handle_alert(&self, _alert: LocalAlertId, _state: AlertState) -> bool {
        #[cfg(feature = "test_alerthandler")]
        {
            //SAFETY: actually safe as the kernel is monothreaded
            unsafe { tests::TEST_ALERTHANDLER.set(_alert) };
        }
        //TODO: actually handle the alert
        true
    }

    /* ALERT CAPSULE */

    /// notify the userspace that an alert happened
    pub fn notify_userspace(&self, alertid: AlertId) {
        // notify the client that an alert happened, if
        self.capsule_ref
            .map(|client| client.alert_happened(alertid as u32));
    }

    /* DEBUGGING */
    /// function used to debug AlertHandler configuration
    pub fn dump_alert_config(&self, start: usize, stop: usize) {
        kernel::debug_verbose!("dump_alert_config");
        for i in start..stop {
            kernel::debug!(
                " cause{} en{} class{} rwen{} <- {} ",
                self.registers.alert_cause[i].get(),
                self.registers.alert_en_shadowed[i].get(),
                self.registers.alert_class_shadowed[i].get(),
                self.registers.alert_regwen[i].get(),
                // TODO report alert name if `top_earlgrey` adds
                // `#[derive(Debug)]` to `AlertId`.
                //
                //AlertId::try_from(i as u32).unwrap(),
                i,
            );
        }
    }

    /// function used to debug AlertHandler configuration for local alerts
    pub fn dump_local_alert_config(&self) {
        kernel::debug_verbose!("dump_local_alert_config");
        for i in 0..ALERTFLAGS_NUMBER_OF_LOCAL_ALERTS {
            kernel::debug!(
                " cause{} en{} class{} rwen{} <- {:?}| ",
                self.registers.loc_alert_cause[i].get(),
                self.registers.loc_alert_en_shadowed[i].get(),
                self.registers.loc_alert_class_shadowed[i].get(),
                self.registers.loc_alert_regwen[i].get(),
                LocalAlertId::try_from(i as u32).unwrap(),
            );
        }
    }
}

#[cfg(feature = "test_alerthandler")]
pub mod tests {
    use core::cell::Cell;

    use crate::alert_handler::{
        AlertClass, AlertFlags, LocalAlertFlags, ALERTFLAGS_NUMBER_OF_ALERTS,
        ALERTFLAGS_NUMBER_OF_LOCAL_ALERTS,
    };
    use crate::registers::alert_handler_regs::ALERT_HANDLER_PARAM_N_ALERTS;
    use crate::registers::top_earlgrey::AlertId;

    use super::AlertHandler;
    use super::LocalAlertId;
    use kernel::hil::time::Alarm;
    use kernel::hil::time::AlarmClient;
    use kernel::hil::time::ConvertTicks;
    use kernel::utilities::cells::OptionalCell;
    use lowrisc::uart::Uart;

    pub static mut TEST_ALERTHANDLER: OptionalCell<LocalAlertId> = OptionalCell::empty();

    #[derive(Clone, Copy)]
    pub enum TestStage {
        TRIGGERS,
        CHECKS,
    }
    pub struct Tests<'a, A: Alarm<'a>> {
        alert_handler: &'a AlertHandler,
        alarm: &'a A,
        uart: &'a Uart<'a>,
        stage: Cell<TestStage>,
    }

    impl<'a, A: Alarm<'a>> Tests<'a, A> {
        pub fn new(alert_handler: &'a AlertHandler, alarm: &'a A, uart: &'a Uart<'a>) -> Self {
            Self {
                alert_handler,
                alarm,
                uart,
                stage: Cell::new(TestStage::TRIGGERS),
            }
        }

        /* TEST ALERTHANDLER FAIL SHADOW REGISTER */
        /// test alert handling functionality by triggering a fault and checking that the correct alert handling functions are called:
        /// - trigger a shadow register update fault
        /// - prepare an alarm (that fires in 100ms from that moment)
        /// - observe as the alert handler is triggered via interrupt
        /// - `AlertHandler::handle_alert` is called
        ///     - TEST_ALERTHANDLER is poppulated with the alert cause
        /// - the alarm fires and checks that `TEST_ALERTHANDLER` is correctly populated
        fn test_alerthandler_fail_shadow_reg(&self) {
            // enable alert detection for Shadow Register Update
            assert!(
                self.alert_handler
                    .configure_local_alert(
                        LocalAlertId::ShadowRegisterUpdateError,
                        super::AlertClass::ClassA,
                        true,
                    )
                    .is_ok(),
                "alert could not be configured, configuration reg is probably locked"
            );

            self.alert_handler.enable_all_interrupts();

            // clear the flag
            // SAFETY: Tock kernel is monothreaded
            unsafe {
                TEST_ALERTHANDLER.insert(None);
            }
            // trigger a local alert (Shadow Register Update Fail)
            self.alert_handler.fail_shadow_reg();

            // test continues in `alarm` function and `check_alerthandler_fail_shadow_reg`
        }

        /// finish `test_alerthandler_fail_shadow_reg()` test by checking that the correct alert handler function was called and that the alert cause flag is lowered
        fn check_alerthandler_fail_shadow_reg(&self) {
            assert!(
                !self
                    .alert_handler
                    .is_local_alert_cause_set(LocalAlertId::ShadowRegisterUpdateError),
                "as the alert was handled, the cause flag should be lowered",
            );

            //check that the alert was handled
            assert_eq!(
                unsafe { TEST_ALERTHANDLER.get() },
                Some(LocalAlertId::ShadowRegisterUpdateError)
            );

            kernel::debug!("TEST AlertHandler ShadowRegister PASS");
        }

        /* TEST AlertFlags */

        /// test 'mark` and `is_set` functions for AlertFlags
        pub fn test_alertflags_base_mark_is_set() {
            // check that setting one flags doesn't influence nearby flags, or the flag in the next boundary region with the same 32 base
            let mut flags_random = AlertFlags::empty();
            let value = 20;
            flags_random.set(value);
            assert!(flags_random.is_set(value));
            assert!(!flags_random.is_set(value + 1));
            assert!(!flags_random.is_set(value - 1));
            assert!(!flags_random.is_set(value + 32));

            // check that setting all of the odd numbered flags doesn't interfere with even numbered flags
            let mut flags_odd_set = AlertFlags::empty();
            for index in 0..ALERTFLAGS_NUMBER_OF_ALERTS / 2 {
                flags_odd_set.set(index * 2 + 1);
            }
            // check first 4 flags
            assert!(!flags_odd_set.is_set(0));
            assert!(flags_odd_set.is_set(1));
            assert!(!flags_odd_set.is_set(2));
            assert!(flags_odd_set.is_set(3));

            // check flags near the first boundary
            assert!(!flags_odd_set.is_set(30));
            assert!(flags_odd_set.is_set(31));
            assert!(!flags_odd_set.is_set(32));
            assert!(flags_odd_set.is_set(33));

            // check flags near the second boundary
            assert!(!flags_odd_set.is_set(62));
            assert!(flags_odd_set.is_set(63));
            assert!(!flags_odd_set.is_set(64));
        }

        /// test 'mark` and `is_set` functions for LocalAlertFlags
        pub fn test_localalertflags_base_mark_is_set() {
            // check that setting one flags doesn't influence nearby flags
            let mut flags_random = LocalAlertFlags::empty();
            let value = 5;
            flags_random.set(value);
            assert!(flags_random.is_set(value));
            assert!(!flags_random.is_set(value + 1));
            assert!(!flags_random.is_set(value - 1));

            // check that setting all of the odd numbered flags doesn't interfere with even numbered flags
            let mut flags_odd_set = LocalAlertFlags::empty();
            for index in 0..ALERTFLAGS_NUMBER_OF_LOCAL_ALERTS / 2 {
                flags_odd_set.set(index * 2 + 1);
            }

            // check first 4 flags
            assert!(!flags_odd_set.is_set(0));
            assert!(flags_odd_set.is_set(1));
            assert!(!flags_odd_set.is_set(2));
            assert!(flags_odd_set.is_set(3));
            assert!(!flags_odd_set.is_set(4));
            assert!(flags_odd_set.is_set(5));
            assert!(!flags_odd_set.is_set(6));
        }

        /// test `for_each_new`` function by applying it to a field of flags previously handled (`flags_test`) and a field with all fields set (the current state of the flags). The function should call the closure only on the flags that are set in `flags_all_set` and are not set in `flags_test`.
        /// inputs:
        ///  - `flags_test` contain the flags that have been previously handled (flags with odd id)
        ///  - `flags_all_set` contains the flags that are currently raised  (all flags)
        /// outputs:
        ///  - `flags_test` will contain the flags that have been previously handled and now are handled (flags with odd id + all flags)
        ///  - `flags_called` will contain all the flags that are handled in this test function (flags with odd if)
        ///  - `anything_new` will signal if at least one flag was handled

        /// ```ignore
        ///                  012345
        ///       flags_test FTFTFT   (previously handled)
        ///    flags_all_set TTTTTT   (currently raised)
        /// ----------------------
        /// flags_test (out) TTTTTT   (now all flags have been handled)
        ///     flags_called TFTFTF   (in this cycle only these flags should be called)
        /// ```

        pub fn test_alertflags_for_each_new() {
            // prepare test variables
            let mut flags_odd_set = AlertFlags::empty();
            let mut flags_even_set = AlertFlags::empty();
            let mut flags_all_set = AlertFlags::empty();
            let mut flags_called = AlertFlags::empty();

            for index in 0..ALERT_HANDLER_PARAM_N_ALERTS as usize / 2 {
                flags_odd_set.set(index * 2 + 1);
                flags_even_set.set(index * 2);
                flags_all_set.set(index * 2);
                flags_all_set.set(index * 2 + 1);
            }

            let mut flags_test = flags_odd_set;

            // call function under test
            let anything_new = flags_test.for_each_new(&flags_all_set, |id| {
                // check if the flag was handled already in this cycle
                if flags_called.is_set(id as usize) {
                    panic!("this flag was already handled in this cycle");
                } else {
                    flags_called.set(id as usize)
                }
            });

            // `flags_test` shoudl have all flags set as they were either set previousely in `flags_test` or were handled now
            assert_eq!(
                flags_test, flags_all_set,
                "result should have all flags set"
            );

            assert!(anything_new, "more than zero flags were changed but the function returned that there were no changes");

            // check that calls were made only to functions with flags that have NOT been previousely handled and have flags currently raised
            assert_eq!(
                flags_called, flags_even_set,
                "number of set flags should be 32"
            );
        }

        /// repeat `test_alertflags_for_each_new` for LocalAlertFlags
        pub fn test_localalertflags_for_each_new() {
            // prepare test variables
            let mut flags_odd_set = LocalAlertFlags::empty();
            let mut flags_even_set = LocalAlertFlags::empty();
            let mut flags_all_set = LocalAlertFlags::empty();
            let mut flags_called = LocalAlertFlags::empty();

            for index in 0..ALERTFLAGS_NUMBER_OF_LOCAL_ALERTS / 2 {
                flags_odd_set.set(index * 2 + 1);
                flags_even_set.set(index * 2);
                flags_all_set.set(index * 2);
                flags_all_set.set(index * 2 + 1);
            }

            let mut flags_test = flags_odd_set;

            // call function under test
            let anything_new = flags_test.for_each_new(&flags_all_set, |id| {
                // check if the flag was handled already in this cycle
                if flags_called.is_set(id as usize) {
                    panic!("this flag was already handled in this cycle");
                } else {
                    flags_called.set(id as usize)
                }
            });

            // `flags_test` shoudl have all flags set as they were either set previousely in `flags_test` or were handled now
            assert_eq!(
                flags_test, flags_all_set,
                "result should have all flags set"
            );

            assert!(anything_new, "more than zero flags were changed but the function returned that there were no changes");

            // check that calls were made only to functions with flags that have NOT been previousely handled and have flags currently raised
            assert_eq!(
                flags_called, flags_even_set,
                "number of set flags should be 32"
            );
        }

        /// test `for_each_new` function's behaviour when no new flags are set. No closure should be called, `anything_new` should be false.
        fn test_alertflags_no_new_flags() {
            // prepare test inputs (only odd flags set)
            let mut handled_flags = AlertFlags::empty();
            let mut new_flags = AlertFlags::empty();
            for index in 0..ALERT_HANDLER_PARAM_N_ALERTS as usize / 2 {
                handled_flags.set(index * 2 + 1);
                new_flags.set(index * 2 + 1);
            }

            // execute function under test
            let anything_new = handled_flags.for_each_new(&new_flags, |a| {
                // this closure shouldn' have been called as no new flags are set
                panic!("this flag {:?} shouldn't have been handled", a)
            });

            // function under test should return that no new flags have been handled
            assert!(
                !anything_new,
                "`for_each_new` should have returned false as no new flags have been raised"
            );
        }

        /// execute `test_alertflags_no_new_flags` for `LocalAlertFlags`
        fn test_localalertflags_no_new_flags() {
            let mut hanlded_localflags = LocalAlertFlags::empty();
            let mut new_localflags = LocalAlertFlags::empty();
            for index in 0..ALERTFLAGS_NUMBER_OF_LOCAL_ALERTS / 2 {
                hanlded_localflags.set(index * 2 + 1);
                new_localflags.set(index * 2 + 1);
            }

            let anything_new = hanlded_localflags.for_each_new(&new_localflags, |a| {
                panic!("this flag {:?} shouldn't have been handled", a);
            });

            assert!(
                !anything_new,
                "'for_each_new` should have returned false as not new flags have been raised"
            );
        }

        /* TEST ALERTHANDLER UART FatalFault */
        /// test alert handling functionality by triggering a fault and checking that the correct alert handling functions are called:
        /// - trigger a UART FatalFault:
        /// - prepare an alarm (that fires in 100ms from that moment)
        /// - observe as the alert handler is triggered via interrupt
        /// - `AlertHandler::handle_alert` is called
        ///     - TEST_ALERTHANDLER_UART is set
        /// - the alarm fires and checks that `TEST_ALERTHANDLER_UART` is correctly populated
        fn test_alerthandler_uartfatalfault(&self) {
            assert!(
                self.alert_handler
                    .configure_alert(AlertId::Uart0FatalFault, AlertClass::ClassA, false)
                    .is_ok(),
                "alert could not be configured, configuration reg is probably locked"
            );

            self.alert_handler.enable_all_interrupts();

            // clear the flag
            // SAFETY: Tock kernel is monothreaded
            unsafe {
                lowrisc::uart::tests::TEST_ALERTHANDLER_UART.set(0);
            }
            // trigger an alert
            self.uart.test_alert();

            // test continues in `alarm` function and `check_alerthandler_uartfatalfault`
        }

        /// finish `test_alerthandler_uartfatalfault()` test by checking that the correct alert handler function was called and that the alert cause flag is lowered
        fn check_alerthandler_uartfatalfault(&self) {
            // check that the alert was handled
            assert_eq!(
                unsafe { lowrisc::uart::tests::TEST_ALERTHANDLER_UART.get() },
                Some(1),
                " alert was not handled as this flag should have been modified"
            );

            assert!(
                !self
                    .alert_handler
                    .is_alert_cause_set(AlertId::Uart0FatalFault),
                "as the alert was handled, the cause flag should be lowered"
            );

            kernel::debug!("TEST AlertHandler UARTFatalFault PASS")
        }

        pub fn run_tests(&self) {
            // run first stage tests in 100ms, more than enough time to start userspace application that might want to listen for generated test alerts
            self.alarm
                .set_alarm(self.alarm.now(), self.alarm.ticks_from_ms(100));
        }
    }

    impl<'a, A: Alarm<'a>> AlarmClient for Tests<'a, A> {
        fn alarm(&self) {
            match self.stage.get() {
                // first stage tests
                TestStage::TRIGGERS => {
                    // unit test for AlertFlags and LocalAlertFlags
                    Self::test_alertflags_base_mark_is_set();
                    Self::test_alertflags_for_each_new();
                    Self::test_alertflags_no_new_flags();
                    Self::test_localalertflags_base_mark_is_set();
                    Self::test_localalertflags_no_new_flags();
                    kernel::debug!("TEST AlertHandler AlertFlags PASS");

                    // test alert handling by generating alerts and observing the generated interrupts
                    self.test_alerthandler_fail_shadow_reg();
                    self.test_alerthandler_uartfatalfault();

                    // prepare an alarm that in 10ms will check if the faults are handled or not
                    self.alarm
                        .set_alarm(self.alarm.now(), self.alarm.ticks_from_ms(10));
                    self.stage.set(TestStage::CHECKS);
                }
                // second stage tests that check if stage 0 triggers have been handled
                TestStage::CHECKS => {
                    self.check_alerthandler_fail_shadow_reg();
                    self.check_alerthandler_uartfatalfault();
                }
            }
        }
    }
}
