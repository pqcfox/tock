// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright lowRISC contributors (OpenTitan project).

// Generated register constants for rv_plic.
// Original reference file: hw/top_earlgrey/ip_autogen/rv_plic/data/rv_plic.hjson
use kernel::utilities::registers::ReadWrite;
use kernel::utilities::registers::{register_bitfields, register_structs};
/// Number of interrupt sources
pub const RV_PLIC_PARAM_NUM_SRC: u32 = 186;
/// Number of Targets (Harts)
pub const RV_PLIC_PARAM_NUM_TARGET: u32 = 1;
/// Width of priority signals
pub const RV_PLIC_PARAM_PRIO_WIDTH: u32 = 2;
/// Number of alerts
pub const RV_PLIC_PARAM_NUM_ALERTS: u32 = 1;
/// Register width
pub const RV_PLIC_PARAM_REG_WIDTH: u32 = 32;

register_structs! {
    pub RvPlicRegisters {
        /// Interrupt Source 0 Priority
        (0x0000 => pub prio0: ReadWrite<u32, PRIO0::Register>),
        /// Interrupt Source 1 Priority
        (0x0004 => pub prio1: ReadWrite<u32, PRIO1::Register>),
        /// Interrupt Source 2 Priority
        (0x0008 => pub prio2: ReadWrite<u32, PRIO2::Register>),
        /// Interrupt Source 3 Priority
        (0x000c => pub prio3: ReadWrite<u32, PRIO3::Register>),
        /// Interrupt Source 4 Priority
        (0x0010 => pub prio4: ReadWrite<u32, PRIO4::Register>),
        /// Interrupt Source 5 Priority
        (0x0014 => pub prio5: ReadWrite<u32, PRIO5::Register>),
        /// Interrupt Source 6 Priority
        (0x0018 => pub prio6: ReadWrite<u32, PRIO6::Register>),
        /// Interrupt Source 7 Priority
        (0x001c => pub prio7: ReadWrite<u32, PRIO7::Register>),
        /// Interrupt Source 8 Priority
        (0x0020 => pub prio8: ReadWrite<u32, PRIO8::Register>),
        /// Interrupt Source 9 Priority
        (0x0024 => pub prio9: ReadWrite<u32, PRIO9::Register>),
        /// Interrupt Source 10 Priority
        (0x0028 => pub prio10: ReadWrite<u32, PRIO10::Register>),
        /// Interrupt Source 11 Priority
        (0x002c => pub prio11: ReadWrite<u32, PRIO11::Register>),
        /// Interrupt Source 12 Priority
        (0x0030 => pub prio12: ReadWrite<u32, PRIO12::Register>),
        /// Interrupt Source 13 Priority
        (0x0034 => pub prio13: ReadWrite<u32, PRIO13::Register>),
        /// Interrupt Source 14 Priority
        (0x0038 => pub prio14: ReadWrite<u32, PRIO14::Register>),
        /// Interrupt Source 15 Priority
        (0x003c => pub prio15: ReadWrite<u32, PRIO15::Register>),
        /// Interrupt Source 16 Priority
        (0x0040 => pub prio16: ReadWrite<u32, PRIO16::Register>),
        /// Interrupt Source 17 Priority
        (0x0044 => pub prio17: ReadWrite<u32, PRIO17::Register>),
        /// Interrupt Source 18 Priority
        (0x0048 => pub prio18: ReadWrite<u32, PRIO18::Register>),
        /// Interrupt Source 19 Priority
        (0x004c => pub prio19: ReadWrite<u32, PRIO19::Register>),
        /// Interrupt Source 20 Priority
        (0x0050 => pub prio20: ReadWrite<u32, PRIO20::Register>),
        /// Interrupt Source 21 Priority
        (0x0054 => pub prio21: ReadWrite<u32, PRIO21::Register>),
        /// Interrupt Source 22 Priority
        (0x0058 => pub prio22: ReadWrite<u32, PRIO22::Register>),
        /// Interrupt Source 23 Priority
        (0x005c => pub prio23: ReadWrite<u32, PRIO23::Register>),
        /// Interrupt Source 24 Priority
        (0x0060 => pub prio24: ReadWrite<u32, PRIO24::Register>),
        /// Interrupt Source 25 Priority
        (0x0064 => pub prio25: ReadWrite<u32, PRIO25::Register>),
        /// Interrupt Source 26 Priority
        (0x0068 => pub prio26: ReadWrite<u32, PRIO26::Register>),
        /// Interrupt Source 27 Priority
        (0x006c => pub prio27: ReadWrite<u32, PRIO27::Register>),
        /// Interrupt Source 28 Priority
        (0x0070 => pub prio28: ReadWrite<u32, PRIO28::Register>),
        /// Interrupt Source 29 Priority
        (0x0074 => pub prio29: ReadWrite<u32, PRIO29::Register>),
        /// Interrupt Source 30 Priority
        (0x0078 => pub prio30: ReadWrite<u32, PRIO30::Register>),
        /// Interrupt Source 31 Priority
        (0x007c => pub prio31: ReadWrite<u32, PRIO31::Register>),
        /// Interrupt Source 32 Priority
        (0x0080 => pub prio32: ReadWrite<u32, PRIO32::Register>),
        /// Interrupt Source 33 Priority
        (0x0084 => pub prio33: ReadWrite<u32, PRIO33::Register>),
        /// Interrupt Source 34 Priority
        (0x0088 => pub prio34: ReadWrite<u32, PRIO34::Register>),
        /// Interrupt Source 35 Priority
        (0x008c => pub prio35: ReadWrite<u32, PRIO35::Register>),
        /// Interrupt Source 36 Priority
        (0x0090 => pub prio36: ReadWrite<u32, PRIO36::Register>),
        /// Interrupt Source 37 Priority
        (0x0094 => pub prio37: ReadWrite<u32, PRIO37::Register>),
        /// Interrupt Source 38 Priority
        (0x0098 => pub prio38: ReadWrite<u32, PRIO38::Register>),
        /// Interrupt Source 39 Priority
        (0x009c => pub prio39: ReadWrite<u32, PRIO39::Register>),
        /// Interrupt Source 40 Priority
        (0x00a0 => pub prio40: ReadWrite<u32, PRIO40::Register>),
        /// Interrupt Source 41 Priority
        (0x00a4 => pub prio41: ReadWrite<u32, PRIO41::Register>),
        /// Interrupt Source 42 Priority
        (0x00a8 => pub prio42: ReadWrite<u32, PRIO42::Register>),
        /// Interrupt Source 43 Priority
        (0x00ac => pub prio43: ReadWrite<u32, PRIO43::Register>),
        /// Interrupt Source 44 Priority
        (0x00b0 => pub prio44: ReadWrite<u32, PRIO44::Register>),
        /// Interrupt Source 45 Priority
        (0x00b4 => pub prio45: ReadWrite<u32, PRIO45::Register>),
        /// Interrupt Source 46 Priority
        (0x00b8 => pub prio46: ReadWrite<u32, PRIO46::Register>),
        /// Interrupt Source 47 Priority
        (0x00bc => pub prio47: ReadWrite<u32, PRIO47::Register>),
        /// Interrupt Source 48 Priority
        (0x00c0 => pub prio48: ReadWrite<u32, PRIO48::Register>),
        /// Interrupt Source 49 Priority
        (0x00c4 => pub prio49: ReadWrite<u32, PRIO49::Register>),
        /// Interrupt Source 50 Priority
        (0x00c8 => pub prio50: ReadWrite<u32, PRIO50::Register>),
        /// Interrupt Source 51 Priority
        (0x00cc => pub prio51: ReadWrite<u32, PRIO51::Register>),
        /// Interrupt Source 52 Priority
        (0x00d0 => pub prio52: ReadWrite<u32, PRIO52::Register>),
        /// Interrupt Source 53 Priority
        (0x00d4 => pub prio53: ReadWrite<u32, PRIO53::Register>),
        /// Interrupt Source 54 Priority
        (0x00d8 => pub prio54: ReadWrite<u32, PRIO54::Register>),
        /// Interrupt Source 55 Priority
        (0x00dc => pub prio55: ReadWrite<u32, PRIO55::Register>),
        /// Interrupt Source 56 Priority
        (0x00e0 => pub prio56: ReadWrite<u32, PRIO56::Register>),
        /// Interrupt Source 57 Priority
        (0x00e4 => pub prio57: ReadWrite<u32, PRIO57::Register>),
        /// Interrupt Source 58 Priority
        (0x00e8 => pub prio58: ReadWrite<u32, PRIO58::Register>),
        /// Interrupt Source 59 Priority
        (0x00ec => pub prio59: ReadWrite<u32, PRIO59::Register>),
        /// Interrupt Source 60 Priority
        (0x00f0 => pub prio60: ReadWrite<u32, PRIO60::Register>),
        /// Interrupt Source 61 Priority
        (0x00f4 => pub prio61: ReadWrite<u32, PRIO61::Register>),
        /// Interrupt Source 62 Priority
        (0x00f8 => pub prio62: ReadWrite<u32, PRIO62::Register>),
        /// Interrupt Source 63 Priority
        (0x00fc => pub prio63: ReadWrite<u32, PRIO63::Register>),
        /// Interrupt Source 64 Priority
        (0x0100 => pub prio64: ReadWrite<u32, PRIO64::Register>),
        /// Interrupt Source 65 Priority
        (0x0104 => pub prio65: ReadWrite<u32, PRIO65::Register>),
        /// Interrupt Source 66 Priority
        (0x0108 => pub prio66: ReadWrite<u32, PRIO66::Register>),
        /// Interrupt Source 67 Priority
        (0x010c => pub prio67: ReadWrite<u32, PRIO67::Register>),
        /// Interrupt Source 68 Priority
        (0x0110 => pub prio68: ReadWrite<u32, PRIO68::Register>),
        /// Interrupt Source 69 Priority
        (0x0114 => pub prio69: ReadWrite<u32, PRIO69::Register>),
        /// Interrupt Source 70 Priority
        (0x0118 => pub prio70: ReadWrite<u32, PRIO70::Register>),
        /// Interrupt Source 71 Priority
        (0x011c => pub prio71: ReadWrite<u32, PRIO71::Register>),
        /// Interrupt Source 72 Priority
        (0x0120 => pub prio72: ReadWrite<u32, PRIO72::Register>),
        /// Interrupt Source 73 Priority
        (0x0124 => pub prio73: ReadWrite<u32, PRIO73::Register>),
        /// Interrupt Source 74 Priority
        (0x0128 => pub prio74: ReadWrite<u32, PRIO74::Register>),
        /// Interrupt Source 75 Priority
        (0x012c => pub prio75: ReadWrite<u32, PRIO75::Register>),
        /// Interrupt Source 76 Priority
        (0x0130 => pub prio76: ReadWrite<u32, PRIO76::Register>),
        /// Interrupt Source 77 Priority
        (0x0134 => pub prio77: ReadWrite<u32, PRIO77::Register>),
        /// Interrupt Source 78 Priority
        (0x0138 => pub prio78: ReadWrite<u32, PRIO78::Register>),
        /// Interrupt Source 79 Priority
        (0x013c => pub prio79: ReadWrite<u32, PRIO79::Register>),
        /// Interrupt Source 80 Priority
        (0x0140 => pub prio80: ReadWrite<u32, PRIO80::Register>),
        /// Interrupt Source 81 Priority
        (0x0144 => pub prio81: ReadWrite<u32, PRIO81::Register>),
        /// Interrupt Source 82 Priority
        (0x0148 => pub prio82: ReadWrite<u32, PRIO82::Register>),
        /// Interrupt Source 83 Priority
        (0x014c => pub prio83: ReadWrite<u32, PRIO83::Register>),
        /// Interrupt Source 84 Priority
        (0x0150 => pub prio84: ReadWrite<u32, PRIO84::Register>),
        /// Interrupt Source 85 Priority
        (0x0154 => pub prio85: ReadWrite<u32, PRIO85::Register>),
        /// Interrupt Source 86 Priority
        (0x0158 => pub prio86: ReadWrite<u32, PRIO86::Register>),
        /// Interrupt Source 87 Priority
        (0x015c => pub prio87: ReadWrite<u32, PRIO87::Register>),
        /// Interrupt Source 88 Priority
        (0x0160 => pub prio88: ReadWrite<u32, PRIO88::Register>),
        /// Interrupt Source 89 Priority
        (0x0164 => pub prio89: ReadWrite<u32, PRIO89::Register>),
        /// Interrupt Source 90 Priority
        (0x0168 => pub prio90: ReadWrite<u32, PRIO90::Register>),
        /// Interrupt Source 91 Priority
        (0x016c => pub prio91: ReadWrite<u32, PRIO91::Register>),
        /// Interrupt Source 92 Priority
        (0x0170 => pub prio92: ReadWrite<u32, PRIO92::Register>),
        /// Interrupt Source 93 Priority
        (0x0174 => pub prio93: ReadWrite<u32, PRIO93::Register>),
        /// Interrupt Source 94 Priority
        (0x0178 => pub prio94: ReadWrite<u32, PRIO94::Register>),
        /// Interrupt Source 95 Priority
        (0x017c => pub prio95: ReadWrite<u32, PRIO95::Register>),
        /// Interrupt Source 96 Priority
        (0x0180 => pub prio96: ReadWrite<u32, PRIO96::Register>),
        /// Interrupt Source 97 Priority
        (0x0184 => pub prio97: ReadWrite<u32, PRIO97::Register>),
        /// Interrupt Source 98 Priority
        (0x0188 => pub prio98: ReadWrite<u32, PRIO98::Register>),
        /// Interrupt Source 99 Priority
        (0x018c => pub prio99: ReadWrite<u32, PRIO99::Register>),
        /// Interrupt Source 100 Priority
        (0x0190 => pub prio100: ReadWrite<u32, PRIO100::Register>),
        /// Interrupt Source 101 Priority
        (0x0194 => pub prio101: ReadWrite<u32, PRIO101::Register>),
        /// Interrupt Source 102 Priority
        (0x0198 => pub prio102: ReadWrite<u32, PRIO102::Register>),
        /// Interrupt Source 103 Priority
        (0x019c => pub prio103: ReadWrite<u32, PRIO103::Register>),
        /// Interrupt Source 104 Priority
        (0x01a0 => pub prio104: ReadWrite<u32, PRIO104::Register>),
        /// Interrupt Source 105 Priority
        (0x01a4 => pub prio105: ReadWrite<u32, PRIO105::Register>),
        /// Interrupt Source 106 Priority
        (0x01a8 => pub prio106: ReadWrite<u32, PRIO106::Register>),
        /// Interrupt Source 107 Priority
        (0x01ac => pub prio107: ReadWrite<u32, PRIO107::Register>),
        /// Interrupt Source 108 Priority
        (0x01b0 => pub prio108: ReadWrite<u32, PRIO108::Register>),
        /// Interrupt Source 109 Priority
        (0x01b4 => pub prio109: ReadWrite<u32, PRIO109::Register>),
        /// Interrupt Source 110 Priority
        (0x01b8 => pub prio110: ReadWrite<u32, PRIO110::Register>),
        /// Interrupt Source 111 Priority
        (0x01bc => pub prio111: ReadWrite<u32, PRIO111::Register>),
        /// Interrupt Source 112 Priority
        (0x01c0 => pub prio112: ReadWrite<u32, PRIO112::Register>),
        /// Interrupt Source 113 Priority
        (0x01c4 => pub prio113: ReadWrite<u32, PRIO113::Register>),
        /// Interrupt Source 114 Priority
        (0x01c8 => pub prio114: ReadWrite<u32, PRIO114::Register>),
        /// Interrupt Source 115 Priority
        (0x01cc => pub prio115: ReadWrite<u32, PRIO115::Register>),
        /// Interrupt Source 116 Priority
        (0x01d0 => pub prio116: ReadWrite<u32, PRIO116::Register>),
        /// Interrupt Source 117 Priority
        (0x01d4 => pub prio117: ReadWrite<u32, PRIO117::Register>),
        /// Interrupt Source 118 Priority
        (0x01d8 => pub prio118: ReadWrite<u32, PRIO118::Register>),
        /// Interrupt Source 119 Priority
        (0x01dc => pub prio119: ReadWrite<u32, PRIO119::Register>),
        /// Interrupt Source 120 Priority
        (0x01e0 => pub prio120: ReadWrite<u32, PRIO120::Register>),
        /// Interrupt Source 121 Priority
        (0x01e4 => pub prio121: ReadWrite<u32, PRIO121::Register>),
        /// Interrupt Source 122 Priority
        (0x01e8 => pub prio122: ReadWrite<u32, PRIO122::Register>),
        /// Interrupt Source 123 Priority
        (0x01ec => pub prio123: ReadWrite<u32, PRIO123::Register>),
        /// Interrupt Source 124 Priority
        (0x01f0 => pub prio124: ReadWrite<u32, PRIO124::Register>),
        /// Interrupt Source 125 Priority
        (0x01f4 => pub prio125: ReadWrite<u32, PRIO125::Register>),
        /// Interrupt Source 126 Priority
        (0x01f8 => pub prio126: ReadWrite<u32, PRIO126::Register>),
        /// Interrupt Source 127 Priority
        (0x01fc => pub prio127: ReadWrite<u32, PRIO127::Register>),
        /// Interrupt Source 128 Priority
        (0x0200 => pub prio128: ReadWrite<u32, PRIO128::Register>),
        /// Interrupt Source 129 Priority
        (0x0204 => pub prio129: ReadWrite<u32, PRIO129::Register>),
        /// Interrupt Source 130 Priority
        (0x0208 => pub prio130: ReadWrite<u32, PRIO130::Register>),
        /// Interrupt Source 131 Priority
        (0x020c => pub prio131: ReadWrite<u32, PRIO131::Register>),
        /// Interrupt Source 132 Priority
        (0x0210 => pub prio132: ReadWrite<u32, PRIO132::Register>),
        /// Interrupt Source 133 Priority
        (0x0214 => pub prio133: ReadWrite<u32, PRIO133::Register>),
        /// Interrupt Source 134 Priority
        (0x0218 => pub prio134: ReadWrite<u32, PRIO134::Register>),
        /// Interrupt Source 135 Priority
        (0x021c => pub prio135: ReadWrite<u32, PRIO135::Register>),
        /// Interrupt Source 136 Priority
        (0x0220 => pub prio136: ReadWrite<u32, PRIO136::Register>),
        /// Interrupt Source 137 Priority
        (0x0224 => pub prio137: ReadWrite<u32, PRIO137::Register>),
        /// Interrupt Source 138 Priority
        (0x0228 => pub prio138: ReadWrite<u32, PRIO138::Register>),
        /// Interrupt Source 139 Priority
        (0x022c => pub prio139: ReadWrite<u32, PRIO139::Register>),
        /// Interrupt Source 140 Priority
        (0x0230 => pub prio140: ReadWrite<u32, PRIO140::Register>),
        /// Interrupt Source 141 Priority
        (0x0234 => pub prio141: ReadWrite<u32, PRIO141::Register>),
        /// Interrupt Source 142 Priority
        (0x0238 => pub prio142: ReadWrite<u32, PRIO142::Register>),
        /// Interrupt Source 143 Priority
        (0x023c => pub prio143: ReadWrite<u32, PRIO143::Register>),
        /// Interrupt Source 144 Priority
        (0x0240 => pub prio144: ReadWrite<u32, PRIO144::Register>),
        /// Interrupt Source 145 Priority
        (0x0244 => pub prio145: ReadWrite<u32, PRIO145::Register>),
        /// Interrupt Source 146 Priority
        (0x0248 => pub prio146: ReadWrite<u32, PRIO146::Register>),
        /// Interrupt Source 147 Priority
        (0x024c => pub prio147: ReadWrite<u32, PRIO147::Register>),
        /// Interrupt Source 148 Priority
        (0x0250 => pub prio148: ReadWrite<u32, PRIO148::Register>),
        /// Interrupt Source 149 Priority
        (0x0254 => pub prio149: ReadWrite<u32, PRIO149::Register>),
        /// Interrupt Source 150 Priority
        (0x0258 => pub prio150: ReadWrite<u32, PRIO150::Register>),
        /// Interrupt Source 151 Priority
        (0x025c => pub prio151: ReadWrite<u32, PRIO151::Register>),
        /// Interrupt Source 152 Priority
        (0x0260 => pub prio152: ReadWrite<u32, PRIO152::Register>),
        /// Interrupt Source 153 Priority
        (0x0264 => pub prio153: ReadWrite<u32, PRIO153::Register>),
        /// Interrupt Source 154 Priority
        (0x0268 => pub prio154: ReadWrite<u32, PRIO154::Register>),
        /// Interrupt Source 155 Priority
        (0x026c => pub prio155: ReadWrite<u32, PRIO155::Register>),
        /// Interrupt Source 156 Priority
        (0x0270 => pub prio156: ReadWrite<u32, PRIO156::Register>),
        /// Interrupt Source 157 Priority
        (0x0274 => pub prio157: ReadWrite<u32, PRIO157::Register>),
        /// Interrupt Source 158 Priority
        (0x0278 => pub prio158: ReadWrite<u32, PRIO158::Register>),
        /// Interrupt Source 159 Priority
        (0x027c => pub prio159: ReadWrite<u32, PRIO159::Register>),
        /// Interrupt Source 160 Priority
        (0x0280 => pub prio160: ReadWrite<u32, PRIO160::Register>),
        /// Interrupt Source 161 Priority
        (0x0284 => pub prio161: ReadWrite<u32, PRIO161::Register>),
        /// Interrupt Source 162 Priority
        (0x0288 => pub prio162: ReadWrite<u32, PRIO162::Register>),
        /// Interrupt Source 163 Priority
        (0x028c => pub prio163: ReadWrite<u32, PRIO163::Register>),
        /// Interrupt Source 164 Priority
        (0x0290 => pub prio164: ReadWrite<u32, PRIO164::Register>),
        /// Interrupt Source 165 Priority
        (0x0294 => pub prio165: ReadWrite<u32, PRIO165::Register>),
        /// Interrupt Source 166 Priority
        (0x0298 => pub prio166: ReadWrite<u32, PRIO166::Register>),
        /// Interrupt Source 167 Priority
        (0x029c => pub prio167: ReadWrite<u32, PRIO167::Register>),
        /// Interrupt Source 168 Priority
        (0x02a0 => pub prio168: ReadWrite<u32, PRIO168::Register>),
        /// Interrupt Source 169 Priority
        (0x02a4 => pub prio169: ReadWrite<u32, PRIO169::Register>),
        /// Interrupt Source 170 Priority
        (0x02a8 => pub prio170: ReadWrite<u32, PRIO170::Register>),
        /// Interrupt Source 171 Priority
        (0x02ac => pub prio171: ReadWrite<u32, PRIO171::Register>),
        /// Interrupt Source 172 Priority
        (0x02b0 => pub prio172: ReadWrite<u32, PRIO172::Register>),
        /// Interrupt Source 173 Priority
        (0x02b4 => pub prio173: ReadWrite<u32, PRIO173::Register>),
        /// Interrupt Source 174 Priority
        (0x02b8 => pub prio174: ReadWrite<u32, PRIO174::Register>),
        /// Interrupt Source 175 Priority
        (0x02bc => pub prio175: ReadWrite<u32, PRIO175::Register>),
        /// Interrupt Source 176 Priority
        (0x02c0 => pub prio176: ReadWrite<u32, PRIO176::Register>),
        /// Interrupt Source 177 Priority
        (0x02c4 => pub prio177: ReadWrite<u32, PRIO177::Register>),
        /// Interrupt Source 178 Priority
        (0x02c8 => pub prio178: ReadWrite<u32, PRIO178::Register>),
        /// Interrupt Source 179 Priority
        (0x02cc => pub prio179: ReadWrite<u32, PRIO179::Register>),
        /// Interrupt Source 180 Priority
        (0x02d0 => pub prio180: ReadWrite<u32, PRIO180::Register>),
        /// Interrupt Source 181 Priority
        (0x02d4 => pub prio181: ReadWrite<u32, PRIO181::Register>),
        /// Interrupt Source 182 Priority
        (0x02d8 => pub prio182: ReadWrite<u32, PRIO182::Register>),
        /// Interrupt Source 183 Priority
        (0x02dc => pub prio183: ReadWrite<u32, PRIO183::Register>),
        /// Interrupt Source 184 Priority
        (0x02e0 => pub prio184: ReadWrite<u32, PRIO184::Register>),
        /// Interrupt Source 185 Priority
        (0x02e4 => pub prio185: ReadWrite<u32, PRIO185::Register>),
        (0x02e8 => _reserved1),
        /// Interrupt Pending
        (0x1000 => pub ip: [ReadWrite<u32, IP::Register>; 6]),
        (0x1018 => _reserved2),
        /// Interrupt Enable for Target 0
        (0x2000 => pub ie0: [ReadWrite<u32, IE0::Register>; 6]),
        (0x2018 => _reserved3),
        /// Threshold of priority for Target 0
        (0x200000 => pub threshold0: ReadWrite<u32, THRESHOLD0::Register>),
        /// Claim interrupt by read, complete interrupt by write for Target 0.
        (0x200004 => pub cc0: ReadWrite<u32, CC0::Register>),
        (0x200008 => _reserved4),
        /// msip for Hart 0.
        (0x4000000 => pub msip0: ReadWrite<u32, MSIP0::Register>),
        (0x4000004 => _reserved5),
        /// Alert Test Register.
        (0x4004000 => pub alert_test: ReadWrite<u32, ALERT_TEST::Register>),
        (0x4004004 => @END),
    }
}

register_bitfields![u32,
    pub PRIO0 [
        PRIO0 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO1 [
        PRIO1 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO2 [
        PRIO2 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO3 [
        PRIO3 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO4 [
        PRIO4 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO5 [
        PRIO5 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO6 [
        PRIO6 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO7 [
        PRIO7 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO8 [
        PRIO8 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO9 [
        PRIO9 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO10 [
        PRIO10 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO11 [
        PRIO11 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO12 [
        PRIO12 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO13 [
        PRIO13 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO14 [
        PRIO14 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO15 [
        PRIO15 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO16 [
        PRIO16 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO17 [
        PRIO17 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO18 [
        PRIO18 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO19 [
        PRIO19 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO20 [
        PRIO20 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO21 [
        PRIO21 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO22 [
        PRIO22 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO23 [
        PRIO23 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO24 [
        PRIO24 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO25 [
        PRIO25 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO26 [
        PRIO26 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO27 [
        PRIO27 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO28 [
        PRIO28 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO29 [
        PRIO29 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO30 [
        PRIO30 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO31 [
        PRIO31 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO32 [
        PRIO32 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO33 [
        PRIO33 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO34 [
        PRIO34 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO35 [
        PRIO35 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO36 [
        PRIO36 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO37 [
        PRIO37 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO38 [
        PRIO38 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO39 [
        PRIO39 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO40 [
        PRIO40 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO41 [
        PRIO41 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO42 [
        PRIO42 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO43 [
        PRIO43 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO44 [
        PRIO44 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO45 [
        PRIO45 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO46 [
        PRIO46 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO47 [
        PRIO47 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO48 [
        PRIO48 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO49 [
        PRIO49 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO50 [
        PRIO50 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO51 [
        PRIO51 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO52 [
        PRIO52 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO53 [
        PRIO53 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO54 [
        PRIO54 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO55 [
        PRIO55 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO56 [
        PRIO56 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO57 [
        PRIO57 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO58 [
        PRIO58 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO59 [
        PRIO59 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO60 [
        PRIO60 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO61 [
        PRIO61 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO62 [
        PRIO62 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO63 [
        PRIO63 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO64 [
        PRIO64 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO65 [
        PRIO65 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO66 [
        PRIO66 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO67 [
        PRIO67 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO68 [
        PRIO68 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO69 [
        PRIO69 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO70 [
        PRIO70 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO71 [
        PRIO71 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO72 [
        PRIO72 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO73 [
        PRIO73 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO74 [
        PRIO74 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO75 [
        PRIO75 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO76 [
        PRIO76 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO77 [
        PRIO77 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO78 [
        PRIO78 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO79 [
        PRIO79 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO80 [
        PRIO80 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO81 [
        PRIO81 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO82 [
        PRIO82 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO83 [
        PRIO83 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO84 [
        PRIO84 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO85 [
        PRIO85 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO86 [
        PRIO86 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO87 [
        PRIO87 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO88 [
        PRIO88 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO89 [
        PRIO89 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO90 [
        PRIO90 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO91 [
        PRIO91 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO92 [
        PRIO92 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO93 [
        PRIO93 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO94 [
        PRIO94 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO95 [
        PRIO95 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO96 [
        PRIO96 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO97 [
        PRIO97 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO98 [
        PRIO98 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO99 [
        PRIO99 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO100 [
        PRIO100 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO101 [
        PRIO101 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO102 [
        PRIO102 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO103 [
        PRIO103 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO104 [
        PRIO104 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO105 [
        PRIO105 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO106 [
        PRIO106 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO107 [
        PRIO107 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO108 [
        PRIO108 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO109 [
        PRIO109 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO110 [
        PRIO110 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO111 [
        PRIO111 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO112 [
        PRIO112 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO113 [
        PRIO113 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO114 [
        PRIO114 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO115 [
        PRIO115 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO116 [
        PRIO116 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO117 [
        PRIO117 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO118 [
        PRIO118 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO119 [
        PRIO119 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO120 [
        PRIO120 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO121 [
        PRIO121 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO122 [
        PRIO122 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO123 [
        PRIO123 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO124 [
        PRIO124 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO125 [
        PRIO125 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO126 [
        PRIO126 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO127 [
        PRIO127 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO128 [
        PRIO128 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO129 [
        PRIO129 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO130 [
        PRIO130 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO131 [
        PRIO131 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO132 [
        PRIO132 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO133 [
        PRIO133 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO134 [
        PRIO134 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO135 [
        PRIO135 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO136 [
        PRIO136 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO137 [
        PRIO137 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO138 [
        PRIO138 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO139 [
        PRIO139 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO140 [
        PRIO140 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO141 [
        PRIO141 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO142 [
        PRIO142 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO143 [
        PRIO143 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO144 [
        PRIO144 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO145 [
        PRIO145 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO146 [
        PRIO146 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO147 [
        PRIO147 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO148 [
        PRIO148 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO149 [
        PRIO149 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO150 [
        PRIO150 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO151 [
        PRIO151 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO152 [
        PRIO152 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO153 [
        PRIO153 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO154 [
        PRIO154 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO155 [
        PRIO155 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO156 [
        PRIO156 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO157 [
        PRIO157 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO158 [
        PRIO158 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO159 [
        PRIO159 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO160 [
        PRIO160 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO161 [
        PRIO161 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO162 [
        PRIO162 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO163 [
        PRIO163 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO164 [
        PRIO164 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO165 [
        PRIO165 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO166 [
        PRIO166 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO167 [
        PRIO167 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO168 [
        PRIO168 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO169 [
        PRIO169 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO170 [
        PRIO170 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO171 [
        PRIO171 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO172 [
        PRIO172 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO173 [
        PRIO173 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO174 [
        PRIO174 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO175 [
        PRIO175 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO176 [
        PRIO176 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO177 [
        PRIO177 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO178 [
        PRIO178 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO179 [
        PRIO179 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO180 [
        PRIO180 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO181 [
        PRIO181 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO182 [
        PRIO182 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO183 [
        PRIO183 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO184 [
        PRIO184 OFFSET(0) NUMBITS(2) [],
    ],
    pub PRIO185 [
        PRIO185 OFFSET(0) NUMBITS(2) [],
    ],
    pub IP [
        P_0 OFFSET(0) NUMBITS(1) [],
        P_1 OFFSET(1) NUMBITS(1) [],
        P_2 OFFSET(2) NUMBITS(1) [],
        P_3 OFFSET(3) NUMBITS(1) [],
        P_4 OFFSET(4) NUMBITS(1) [],
        P_5 OFFSET(5) NUMBITS(1) [],
        P_6 OFFSET(6) NUMBITS(1) [],
        P_7 OFFSET(7) NUMBITS(1) [],
        P_8 OFFSET(8) NUMBITS(1) [],
        P_9 OFFSET(9) NUMBITS(1) [],
        P_10 OFFSET(10) NUMBITS(1) [],
        P_11 OFFSET(11) NUMBITS(1) [],
        P_12 OFFSET(12) NUMBITS(1) [],
        P_13 OFFSET(13) NUMBITS(1) [],
        P_14 OFFSET(14) NUMBITS(1) [],
        P_15 OFFSET(15) NUMBITS(1) [],
        P_16 OFFSET(16) NUMBITS(1) [],
        P_17 OFFSET(17) NUMBITS(1) [],
        P_18 OFFSET(18) NUMBITS(1) [],
        P_19 OFFSET(19) NUMBITS(1) [],
        P_20 OFFSET(20) NUMBITS(1) [],
        P_21 OFFSET(21) NUMBITS(1) [],
        P_22 OFFSET(22) NUMBITS(1) [],
        P_23 OFFSET(23) NUMBITS(1) [],
        P_24 OFFSET(24) NUMBITS(1) [],
        P_25 OFFSET(25) NUMBITS(1) [],
        P_26 OFFSET(26) NUMBITS(1) [],
        P_27 OFFSET(27) NUMBITS(1) [],
        P_28 OFFSET(28) NUMBITS(1) [],
        P_29 OFFSET(29) NUMBITS(1) [],
        P_30 OFFSET(30) NUMBITS(1) [],
        P_31 OFFSET(31) NUMBITS(1) [],
    ],
    pub IE0 [
        E_0 OFFSET(0) NUMBITS(1) [],
        E_1 OFFSET(1) NUMBITS(1) [],
        E_2 OFFSET(2) NUMBITS(1) [],
        E_3 OFFSET(3) NUMBITS(1) [],
        E_4 OFFSET(4) NUMBITS(1) [],
        E_5 OFFSET(5) NUMBITS(1) [],
        E_6 OFFSET(6) NUMBITS(1) [],
        E_7 OFFSET(7) NUMBITS(1) [],
        E_8 OFFSET(8) NUMBITS(1) [],
        E_9 OFFSET(9) NUMBITS(1) [],
        E_10 OFFSET(10) NUMBITS(1) [],
        E_11 OFFSET(11) NUMBITS(1) [],
        E_12 OFFSET(12) NUMBITS(1) [],
        E_13 OFFSET(13) NUMBITS(1) [],
        E_14 OFFSET(14) NUMBITS(1) [],
        E_15 OFFSET(15) NUMBITS(1) [],
        E_16 OFFSET(16) NUMBITS(1) [],
        E_17 OFFSET(17) NUMBITS(1) [],
        E_18 OFFSET(18) NUMBITS(1) [],
        E_19 OFFSET(19) NUMBITS(1) [],
        E_20 OFFSET(20) NUMBITS(1) [],
        E_21 OFFSET(21) NUMBITS(1) [],
        E_22 OFFSET(22) NUMBITS(1) [],
        E_23 OFFSET(23) NUMBITS(1) [],
        E_24 OFFSET(24) NUMBITS(1) [],
        E_25 OFFSET(25) NUMBITS(1) [],
        E_26 OFFSET(26) NUMBITS(1) [],
        E_27 OFFSET(27) NUMBITS(1) [],
        E_28 OFFSET(28) NUMBITS(1) [],
        E_29 OFFSET(29) NUMBITS(1) [],
        E_30 OFFSET(30) NUMBITS(1) [],
        E_31 OFFSET(31) NUMBITS(1) [],
    ],
    pub THRESHOLD0 [
        THRESHOLD0 OFFSET(0) NUMBITS(2) [],
    ],
    pub CC0 [
        CC0 OFFSET(0) NUMBITS(8) [],
    ],
    pub MSIP0 [
        MSIP0 OFFSET(0) NUMBITS(1) [],
    ],
    pub ALERT_TEST [
        FATAL_FAULT OFFSET(0) NUMBITS(1) [],
    ],
];

// End generated register constants for rv_plic
