// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Platform Level Interrupt Control peripheral driver.

use crate::registers::rv_plic_regs::{
    RvPlicRegisters, PRIO0, PRIO1, PRIO10, PRIO100, PRIO101, PRIO102, PRIO103, PRIO104, PRIO105,
    PRIO106, PRIO107, PRIO108, PRIO109, PRIO11, PRIO110, PRIO111, PRIO112, PRIO113, PRIO114,
    PRIO115, PRIO116, PRIO117, PRIO118, PRIO119, PRIO12, PRIO120, PRIO121, PRIO122, PRIO123,
    PRIO124, PRIO125, PRIO126, PRIO127, PRIO128, PRIO129, PRIO13, PRIO130, PRIO131, PRIO132,
    PRIO133, PRIO134, PRIO135, PRIO136, PRIO137, PRIO138, PRIO139, PRIO14, PRIO140, PRIO141,
    PRIO142, PRIO143, PRIO144, PRIO145, PRIO146, PRIO147, PRIO148, PRIO149, PRIO15, PRIO150,
    PRIO151, PRIO152, PRIO153, PRIO154, PRIO155, PRIO156, PRIO157, PRIO158, PRIO159, PRIO16,
    PRIO160, PRIO161, PRIO162, PRIO163, PRIO164, PRIO165, PRIO166, PRIO167, PRIO168, PRIO169,
    PRIO17, PRIO170, PRIO171, PRIO172, PRIO173, PRIO174, PRIO175, PRIO176, PRIO177, PRIO178,
    PRIO179, PRIO18, PRIO180, PRIO181, PRIO182, PRIO183, PRIO184, PRIO185, PRIO19, PRIO2, PRIO20,
    PRIO21, PRIO22, PRIO23, PRIO24, PRIO25, PRIO26, PRIO27, PRIO28, PRIO29, PRIO3, PRIO30, PRIO31,
    PRIO32, PRIO33, PRIO34, PRIO35, PRIO36, PRIO37, PRIO38, PRIO39, PRIO4, PRIO40, PRIO41, PRIO42,
    PRIO43, PRIO44, PRIO45, PRIO46, PRIO47, PRIO48, PRIO49, PRIO5, PRIO50, PRIO51, PRIO52, PRIO53,
    PRIO54, PRIO55, PRIO56, PRIO57, PRIO58, PRIO59, PRIO6, PRIO60, PRIO61, PRIO62, PRIO63, PRIO64,
    PRIO65, PRIO66, PRIO67, PRIO68, PRIO69, PRIO7, PRIO70, PRIO71, PRIO72, PRIO73, PRIO74, PRIO75,
    PRIO76, PRIO77, PRIO78, PRIO79, PRIO8, PRIO80, PRIO81, PRIO82, PRIO83, PRIO84, PRIO85, PRIO86,
    PRIO87, PRIO88, PRIO89, PRIO9, PRIO90, PRIO91, PRIO92, PRIO93, PRIO94, PRIO95, PRIO96, PRIO97,
    PRIO98, PRIO99, THRESHOLD0,
};
use crate::registers::top_earlgrey::RV_PLIC_BASE_ADDR;
use kernel::utilities::cells::VolatileCell;
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::registers::LocalRegisterCopy;
use kernel::utilities::StaticRef;

pub const PLIC_BASE: StaticRef<RvPlicRegisters> =
    unsafe { StaticRef::new(RV_PLIC_BASE_ADDR as *const RvPlicRegisters) };

pub static mut PLIC: Plic = Plic::new(PLIC_BASE);

pub const PLIC_REGS: usize = 6;
pub const PLIC_IRQ_NUM: usize = 186;

pub struct Plic {
    registers: StaticRef<RvPlicRegisters>,
    saved: [VolatileCell<LocalRegisterCopy<u32>>; PLIC_REGS],
}

impl Plic {
    pub const fn new(base: StaticRef<RvPlicRegisters>) -> Self {
        Plic {
            registers: base,
            saved: [
                VolatileCell::new(LocalRegisterCopy::new(0)),
                VolatileCell::new(LocalRegisterCopy::new(0)),
                VolatileCell::new(LocalRegisterCopy::new(0)),
                VolatileCell::new(LocalRegisterCopy::new(0)),
                VolatileCell::new(LocalRegisterCopy::new(0)),
                VolatileCell::new(LocalRegisterCopy::new(0)),
            ],
        }
    }

    /// Clear all pending interrupts.
    pub fn clear_all_pending(&self) {
        unimplemented!()
    }

    /// Enable all interrupts.
    pub fn enable_all(&self) {
        for enable in self.registers.ie0.iter() {
            enable.set(0xFFFF_FFFF);
        }

        // Set the max priority for each interrupt. This is not really used
        // at this point.
        for index in 0..PLIC_IRQ_NUM {
            self.set_priority(index, 3);
        }

        // Accept all interrupts.
        self.registers
            .threshold0
            .write(THRESHOLD0::THRESHOLD0.val(1));
    }

    /// Disable specific interrupt.
    pub fn disable(&self, index: u32) {
        if index >= PLIC_IRQ_NUM as u32 {
            panic!("Invalid IRQ: {}", index);
        };
        let offset = (index / 32) as usize;
        let mask = !(1 << (index % 32));

        self.registers.ie0[offset].set(self.registers.ie0[offset].get() & mask);
    }

    /// Disable all interrupts.
    pub fn disable_all(&self) {
        for enable in self.registers.ie0.iter() {
            enable.set(0);
        }
    }

    /// Get the index (0-256) of the lowest number pending interrupt, or `None` if
    /// none is pending. RISC-V PLIC has a "claim" register which makes it easy
    /// to grab the highest priority pending interrupt.
    pub fn next_pending(&self) -> Option<u32> {
        let claim = self.registers.cc0.get();
        if claim == 0 {
            None
        } else {
            Some(claim)
        }
    }

    /// Save the current interrupt to be handled later
    /// This will save the interrupt at index internally to be handled later.
    /// Interrupts must be disabled before this is called.
    /// Saved interrupts can be retrieved by calling `get_saved_interrupts()`.
    /// Saved interrupts are cleared when `'complete()` is called.
    pub unsafe fn save_interrupt(&self, index: u32) {
        if index >= PLIC_IRQ_NUM as u32 {
            panic!("Invalid IRQ: {}", index);
        };
        let offset = (index / 32) as usize;
        let mask = 1 << (index % 32);

        // OR the current saved state with the new value
        let new_saved = self.saved[offset].get().get() | mask;

        // Set the new state
        self.saved[offset].set(LocalRegisterCopy::new(new_saved));
    }

    /// The `next_pending()` function will only return enabled interrupts.
    /// This function will return a pending interrupt that has been disabled by
    /// `save_interrupt()`.
    pub fn get_saved_interrupts(&self) -> Option<u32> {
        for (i, pending) in self.saved.iter().enumerate() {
            let saved = pending.get().get();
            if saved != 0 {
                return Some(saved.trailing_zeros() + (i as u32 * 32));
            }
        }
        None
    }

    /// Signal that an interrupt is finished being handled. In Tock, this should be
    /// called from the normal main loop (not the interrupt handler).
    /// Interrupts must be disabled before this is called.
    pub unsafe fn complete(&self, index: u32) {
        self.registers.cc0.set(index);
        if index >= PLIC_IRQ_NUM as u32 {
            panic!("Invalid IRQ: {}", index);
        };
        let offset = (index / 32) as usize;
        let mask = !(1 << (index % 32));

        // OR the current saved state with the new value
        let new_saved = self.saved[offset].get().get() & mask;

        // Set the new state
        self.saved[offset].set(LocalRegisterCopy::new(new_saved));
    }

    /// Sets the `index`th interrupt source priority to the given value
    /// [0-3]. The top 6 bits of `val` are ignored.
    fn set_priority(&self, index: usize, val: u8) {
        let bits = u32::from(val & 0b11);
        match index {
            0 => self.registers.prio0.write(PRIO0::PRIO0.val(bits)),
            1 => self.registers.prio1.write(PRIO1::PRIO1.val(bits)),
            2 => self.registers.prio2.write(PRIO2::PRIO2.val(bits)),
            3 => self.registers.prio3.write(PRIO3::PRIO3.val(bits)),
            4 => self.registers.prio4.write(PRIO4::PRIO4.val(bits)),
            5 => self.registers.prio5.write(PRIO5::PRIO5.val(bits)),
            6 => self.registers.prio6.write(PRIO6::PRIO6.val(bits)),
            7 => self.registers.prio7.write(PRIO7::PRIO7.val(bits)),
            8 => self.registers.prio8.write(PRIO8::PRIO8.val(bits)),
            9 => self.registers.prio9.write(PRIO9::PRIO9.val(bits)),
            10 => self.registers.prio10.write(PRIO10::PRIO10.val(bits)),
            11 => self.registers.prio11.write(PRIO11::PRIO11.val(bits)),
            12 => self.registers.prio12.write(PRIO12::PRIO12.val(bits)),
            13 => self.registers.prio13.write(PRIO13::PRIO13.val(bits)),
            14 => self.registers.prio14.write(PRIO14::PRIO14.val(bits)),
            15 => self.registers.prio15.write(PRIO15::PRIO15.val(bits)),
            16 => self.registers.prio16.write(PRIO16::PRIO16.val(bits)),
            17 => self.registers.prio17.write(PRIO17::PRIO17.val(bits)),
            18 => self.registers.prio18.write(PRIO18::PRIO18.val(bits)),
            19 => self.registers.prio19.write(PRIO19::PRIO19.val(bits)),
            20 => self.registers.prio20.write(PRIO20::PRIO20.val(bits)),
            21 => self.registers.prio21.write(PRIO21::PRIO21.val(bits)),
            22 => self.registers.prio22.write(PRIO22::PRIO22.val(bits)),
            23 => self.registers.prio23.write(PRIO23::PRIO23.val(bits)),
            24 => self.registers.prio24.write(PRIO24::PRIO24.val(bits)),
            25 => self.registers.prio25.write(PRIO25::PRIO25.val(bits)),
            26 => self.registers.prio26.write(PRIO26::PRIO26.val(bits)),
            27 => self.registers.prio27.write(PRIO27::PRIO27.val(bits)),
            28 => self.registers.prio28.write(PRIO28::PRIO28.val(bits)),
            29 => self.registers.prio29.write(PRIO29::PRIO29.val(bits)),
            30 => self.registers.prio30.write(PRIO30::PRIO30.val(bits)),
            31 => self.registers.prio31.write(PRIO31::PRIO31.val(bits)),
            32 => self.registers.prio32.write(PRIO32::PRIO32.val(bits)),
            33 => self.registers.prio33.write(PRIO33::PRIO33.val(bits)),
            34 => self.registers.prio34.write(PRIO34::PRIO34.val(bits)),
            35 => self.registers.prio35.write(PRIO35::PRIO35.val(bits)),
            36 => self.registers.prio36.write(PRIO36::PRIO36.val(bits)),
            37 => self.registers.prio37.write(PRIO37::PRIO37.val(bits)),
            38 => self.registers.prio38.write(PRIO38::PRIO38.val(bits)),
            39 => self.registers.prio39.write(PRIO39::PRIO39.val(bits)),
            40 => self.registers.prio40.write(PRIO40::PRIO40.val(bits)),
            41 => self.registers.prio41.write(PRIO41::PRIO41.val(bits)),
            42 => self.registers.prio42.write(PRIO42::PRIO42.val(bits)),
            43 => self.registers.prio43.write(PRIO43::PRIO43.val(bits)),
            44 => self.registers.prio44.write(PRIO44::PRIO44.val(bits)),
            45 => self.registers.prio45.write(PRIO45::PRIO45.val(bits)),
            46 => self.registers.prio46.write(PRIO46::PRIO46.val(bits)),
            47 => self.registers.prio47.write(PRIO47::PRIO47.val(bits)),
            48 => self.registers.prio48.write(PRIO48::PRIO48.val(bits)),
            49 => self.registers.prio49.write(PRIO49::PRIO49.val(bits)),
            50 => self.registers.prio50.write(PRIO50::PRIO50.val(bits)),
            51 => self.registers.prio51.write(PRIO51::PRIO51.val(bits)),
            52 => self.registers.prio52.write(PRIO52::PRIO52.val(bits)),
            53 => self.registers.prio53.write(PRIO53::PRIO53.val(bits)),
            54 => self.registers.prio54.write(PRIO54::PRIO54.val(bits)),
            55 => self.registers.prio55.write(PRIO55::PRIO55.val(bits)),
            56 => self.registers.prio56.write(PRIO56::PRIO56.val(bits)),
            57 => self.registers.prio57.write(PRIO57::PRIO57.val(bits)),
            58 => self.registers.prio58.write(PRIO58::PRIO58.val(bits)),
            59 => self.registers.prio59.write(PRIO59::PRIO59.val(bits)),
            60 => self.registers.prio60.write(PRIO60::PRIO60.val(bits)),
            61 => self.registers.prio61.write(PRIO61::PRIO61.val(bits)),
            62 => self.registers.prio62.write(PRIO62::PRIO62.val(bits)),
            63 => self.registers.prio63.write(PRIO63::PRIO63.val(bits)),
            64 => self.registers.prio64.write(PRIO64::PRIO64.val(bits)),
            65 => self.registers.prio65.write(PRIO65::PRIO65.val(bits)),
            66 => self.registers.prio66.write(PRIO66::PRIO66.val(bits)),
            67 => self.registers.prio67.write(PRIO67::PRIO67.val(bits)),
            68 => self.registers.prio68.write(PRIO68::PRIO68.val(bits)),
            69 => self.registers.prio69.write(PRIO69::PRIO69.val(bits)),
            70 => self.registers.prio70.write(PRIO70::PRIO70.val(bits)),
            71 => self.registers.prio71.write(PRIO71::PRIO71.val(bits)),
            72 => self.registers.prio72.write(PRIO72::PRIO72.val(bits)),
            73 => self.registers.prio73.write(PRIO73::PRIO73.val(bits)),
            74 => self.registers.prio74.write(PRIO74::PRIO74.val(bits)),
            75 => self.registers.prio75.write(PRIO75::PRIO75.val(bits)),
            76 => self.registers.prio76.write(PRIO76::PRIO76.val(bits)),
            77 => self.registers.prio77.write(PRIO77::PRIO77.val(bits)),
            78 => self.registers.prio78.write(PRIO78::PRIO78.val(bits)),
            79 => self.registers.prio79.write(PRIO79::PRIO79.val(bits)),
            80 => self.registers.prio80.write(PRIO80::PRIO80.val(bits)),
            81 => self.registers.prio81.write(PRIO81::PRIO81.val(bits)),
            82 => self.registers.prio82.write(PRIO82::PRIO82.val(bits)),
            83 => self.registers.prio83.write(PRIO83::PRIO83.val(bits)),
            84 => self.registers.prio84.write(PRIO84::PRIO84.val(bits)),
            85 => self.registers.prio85.write(PRIO85::PRIO85.val(bits)),
            86 => self.registers.prio86.write(PRIO86::PRIO86.val(bits)),
            87 => self.registers.prio87.write(PRIO87::PRIO87.val(bits)),
            88 => self.registers.prio88.write(PRIO88::PRIO88.val(bits)),
            89 => self.registers.prio89.write(PRIO89::PRIO89.val(bits)),
            90 => self.registers.prio90.write(PRIO90::PRIO90.val(bits)),
            91 => self.registers.prio91.write(PRIO91::PRIO91.val(bits)),
            92 => self.registers.prio92.write(PRIO92::PRIO92.val(bits)),
            93 => self.registers.prio93.write(PRIO93::PRIO93.val(bits)),
            94 => self.registers.prio94.write(PRIO94::PRIO94.val(bits)),
            95 => self.registers.prio95.write(PRIO95::PRIO95.val(bits)),
            96 => self.registers.prio96.write(PRIO96::PRIO96.val(bits)),
            97 => self.registers.prio97.write(PRIO97::PRIO97.val(bits)),
            98 => self.registers.prio98.write(PRIO98::PRIO98.val(bits)),
            99 => self.registers.prio99.write(PRIO99::PRIO99.val(bits)),
            100 => self.registers.prio100.write(PRIO100::PRIO100.val(bits)),
            101 => self.registers.prio101.write(PRIO101::PRIO101.val(bits)),
            102 => self.registers.prio102.write(PRIO102::PRIO102.val(bits)),
            103 => self.registers.prio103.write(PRIO103::PRIO103.val(bits)),
            104 => self.registers.prio104.write(PRIO104::PRIO104.val(bits)),
            105 => self.registers.prio105.write(PRIO105::PRIO105.val(bits)),
            106 => self.registers.prio106.write(PRIO106::PRIO106.val(bits)),
            107 => self.registers.prio107.write(PRIO107::PRIO107.val(bits)),
            108 => self.registers.prio108.write(PRIO108::PRIO108.val(bits)),
            109 => self.registers.prio109.write(PRIO109::PRIO109.val(bits)),
            110 => self.registers.prio110.write(PRIO110::PRIO110.val(bits)),
            111 => self.registers.prio111.write(PRIO111::PRIO111.val(bits)),
            112 => self.registers.prio112.write(PRIO112::PRIO112.val(bits)),
            113 => self.registers.prio113.write(PRIO113::PRIO113.val(bits)),
            114 => self.registers.prio114.write(PRIO114::PRIO114.val(bits)),
            115 => self.registers.prio115.write(PRIO115::PRIO115.val(bits)),
            116 => self.registers.prio116.write(PRIO116::PRIO116.val(bits)),
            117 => self.registers.prio117.write(PRIO117::PRIO117.val(bits)),
            118 => self.registers.prio118.write(PRIO118::PRIO118.val(bits)),
            119 => self.registers.prio119.write(PRIO119::PRIO119.val(bits)),
            120 => self.registers.prio120.write(PRIO120::PRIO120.val(bits)),
            121 => self.registers.prio121.write(PRIO121::PRIO121.val(bits)),
            122 => self.registers.prio122.write(PRIO122::PRIO122.val(bits)),
            123 => self.registers.prio123.write(PRIO123::PRIO123.val(bits)),
            124 => self.registers.prio124.write(PRIO124::PRIO124.val(bits)),
            125 => self.registers.prio125.write(PRIO125::PRIO125.val(bits)),
            126 => self.registers.prio126.write(PRIO126::PRIO126.val(bits)),
            127 => self.registers.prio127.write(PRIO127::PRIO127.val(bits)),
            128 => self.registers.prio128.write(PRIO128::PRIO128.val(bits)),
            129 => self.registers.prio129.write(PRIO129::PRIO129.val(bits)),
            130 => self.registers.prio130.write(PRIO130::PRIO130.val(bits)),
            131 => self.registers.prio131.write(PRIO131::PRIO131.val(bits)),
            132 => self.registers.prio132.write(PRIO132::PRIO132.val(bits)),
            133 => self.registers.prio133.write(PRIO133::PRIO133.val(bits)),
            134 => self.registers.prio134.write(PRIO134::PRIO134.val(bits)),
            135 => self.registers.prio135.write(PRIO135::PRIO135.val(bits)),
            136 => self.registers.prio136.write(PRIO136::PRIO136.val(bits)),
            137 => self.registers.prio137.write(PRIO137::PRIO137.val(bits)),
            138 => self.registers.prio138.write(PRIO138::PRIO138.val(bits)),
            139 => self.registers.prio139.write(PRIO139::PRIO139.val(bits)),
            140 => self.registers.prio140.write(PRIO140::PRIO140.val(bits)),
            141 => self.registers.prio141.write(PRIO141::PRIO141.val(bits)),
            142 => self.registers.prio142.write(PRIO142::PRIO142.val(bits)),
            143 => self.registers.prio143.write(PRIO143::PRIO143.val(bits)),
            144 => self.registers.prio144.write(PRIO144::PRIO144.val(bits)),
            145 => self.registers.prio145.write(PRIO145::PRIO145.val(bits)),
            146 => self.registers.prio146.write(PRIO146::PRIO146.val(bits)),
            147 => self.registers.prio147.write(PRIO147::PRIO147.val(bits)),
            148 => self.registers.prio148.write(PRIO148::PRIO148.val(bits)),
            149 => self.registers.prio149.write(PRIO149::PRIO149.val(bits)),
            150 => self.registers.prio150.write(PRIO150::PRIO150.val(bits)),
            151 => self.registers.prio151.write(PRIO151::PRIO151.val(bits)),
            152 => self.registers.prio152.write(PRIO152::PRIO152.val(bits)),
            153 => self.registers.prio153.write(PRIO153::PRIO153.val(bits)),
            154 => self.registers.prio154.write(PRIO154::PRIO154.val(bits)),
            155 => self.registers.prio155.write(PRIO155::PRIO155.val(bits)),
            156 => self.registers.prio156.write(PRIO156::PRIO156.val(bits)),
            157 => self.registers.prio157.write(PRIO157::PRIO157.val(bits)),
            158 => self.registers.prio158.write(PRIO158::PRIO158.val(bits)),
            159 => self.registers.prio159.write(PRIO159::PRIO159.val(bits)),
            160 => self.registers.prio160.write(PRIO160::PRIO160.val(bits)),
            161 => self.registers.prio161.write(PRIO161::PRIO161.val(bits)),
            162 => self.registers.prio162.write(PRIO162::PRIO162.val(bits)),
            163 => self.registers.prio163.write(PRIO163::PRIO163.val(bits)),
            164 => self.registers.prio164.write(PRIO164::PRIO164.val(bits)),
            165 => self.registers.prio165.write(PRIO165::PRIO165.val(bits)),
            166 => self.registers.prio166.write(PRIO166::PRIO166.val(bits)),
            167 => self.registers.prio167.write(PRIO167::PRIO167.val(bits)),
            168 => self.registers.prio168.write(PRIO168::PRIO168.val(bits)),
            169 => self.registers.prio169.write(PRIO169::PRIO169.val(bits)),
            170 => self.registers.prio170.write(PRIO170::PRIO170.val(bits)),
            171 => self.registers.prio171.write(PRIO171::PRIO171.val(bits)),
            172 => self.registers.prio172.write(PRIO172::PRIO172.val(bits)),
            173 => self.registers.prio173.write(PRIO173::PRIO173.val(bits)),
            174 => self.registers.prio174.write(PRIO174::PRIO174.val(bits)),
            175 => self.registers.prio175.write(PRIO175::PRIO175.val(bits)),
            176 => self.registers.prio176.write(PRIO176::PRIO176.val(bits)),
            177 => self.registers.prio177.write(PRIO177::PRIO177.val(bits)),
            178 => self.registers.prio178.write(PRIO178::PRIO178.val(bits)),
            179 => self.registers.prio179.write(PRIO179::PRIO179.val(bits)),
            180 => self.registers.prio180.write(PRIO180::PRIO180.val(bits)),
            181 => self.registers.prio181.write(PRIO181::PRIO181.val(bits)),
            182 => self.registers.prio182.write(PRIO182::PRIO182.val(bits)),
            183 => self.registers.prio183.write(PRIO183::PRIO183.val(bits)),
            184 => self.registers.prio184.write(PRIO184::PRIO184.val(bits)),
            185 => self.registers.prio185.write(PRIO185::PRIO185.val(bits)),
            // Out of bounds := no-op, to avoid panicking
            _ => {}
        }
    }
}
