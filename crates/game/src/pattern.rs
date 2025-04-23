use broadsword::{runtime, scanner};

/// Takes an instruction pattern and looks for its location
pub(crate) fn match_instruction_pattern(pattern: &str) -> Option<PatternResult> {
    // Find .text section details since that's where the code lives
    let text_section = runtime::get_module_section_range("eldenring.exe", ".text")
        .or_else(|_| runtime::get_module_section_range("armoredcore6.exe", ".text"))
        .or_else(|_| runtime::get_module_section_range("sekiro.exe", ".text"))
        .or_else(|_| runtime::get_module_section_range("start_protected_game.exe", ".text"))
        .unwrap();

    // Represent search area as a slice
    let scan_slice = unsafe {
        std::slice::from_raw_parts(
            text_section.start as *const u8,
            text_section.end - text_section.start,
        )
    };

    let pattern = scanner::Pattern::from_bit_pattern(pattern).unwrap();

    scanner::simple::scan(scan_slice, &pattern)
        // TODO: this kinda of rebasing can be done in broadsword probably
        .map(|result| PatternResult {
            location: text_section.start + result.location,
            captures: result.captures.into_iter()
                .map(|capture| {
                    PatternCapture {
                        location: text_section.start + capture.location,
                        bytes: capture.bytes,
                    }
                })
                .collect()
        })
}

#[derive(Debug)]
pub(crate) struct PatternResult {
    pub location: usize,
    pub captures: Vec<PatternCapture>,
}

#[derive(Debug)]
pub(crate) struct PatternCapture {
    pub location: usize,
    pub bytes: Vec<u8>,
}

// 1420fbf20 4c 89 44        MOV        qword ptr [RSP + 0x18]=>local_res18,R8
//           24 18
// 1420fbf25 48 89 54        MOV        qword ptr [RSP + 0x10]=>local_res10,RDX
//           24 10
// 1420fbf2a 48 89 4c        MOV        qword ptr [RSP + 0x8]=>local_res8,RCX
//           24 08
// 1420fbf2f 57              PUSH       RDI
// 1420fbf30 48 81 ec        SUB        RSP,0x100
//           00 01 00 00
// 1420fbf37 48 8b fc        MOV        RDI,RSP
// 1420fbf3a b9 40 00        MOV        ECX,0x40
//           00 00
// 1420fbf3f b8 cc cc        MOV        EAX,0xcccccccc
//           cc cc
// 1420fbf44 f3 ab           STOSD.REP  RDI
// 1420fbf46 48 8b 8c        MOV        RCX,qword ptr [RSP + 0x110]=>local_res8
//           24 10 01 
//           00 00
// 1420fbf4e 48 8b 84        MOV        RAX,qword ptr [RSP + 0x110]=>local_res8
//           24 10 01 
//           00 00
pub(crate) const PATCH_OFFSETS_PATTERN: &str = concat!(
    "01001... 10001001 01000100 ..100100 00011000",
    "01001... 10001001 01010100 ..100100 00010000",
    "01001... 10001001 01001100 ..100100 00001000",
    "01010111",
    "01001... 10000001 11101100 00000000 00000001 00000000 00000000",
    "01001... 10001011 11111100",
    "10111001 01000000 00000000 00000000 00000000",
    "10111000 11001100 11001100 11001100 11001100",
    "11110011 10101011",
    "01001... 10001011 10001100 ..100100 00010000 00000001 00000000 00000000",
    "01001... 10001011 10000100 ..100100 00010000 00000001 00000000 00000000",
);

// 142125030 48 89 4c        MOV        qword ptr [RSP + 0x8]=>local_res8,RCX
//           24 08
// 142125035 57              PUSH       RDI
// 142125036 48 81 ec        SUB        RSP,0x130
//           30 01 00 00
// 14212503d 48 8b fc        MOV        RDI,RSP
// 142125040 b9 4c 00        MOV        ECX,0x4c
//           00 00
// 142125045 b8 cc cc        MOV        EAX,0xcccccccc
//           cc cc
// 14212504a f3 ab           STOSD.REP  RDI
// 14212504c 48 8b 8c        MOV        RCX,qword ptr [RSP + 0x140]=>local_res8
//           24 40 01 
//           00 00
// 142125054 48 8b 84        MOV        RAX,qword ptr [RSP + 0x140]=>local_res8
//           24 40 01 
//           00 00
pub(crate) const WTF_FXR_PATTERN: &str = concat!(
    "01001... 10001001 01001100 ..100100 00001000",
    "01010111",
    "01001... 10000001 11101100 00110000 00000001 00000000 00000000",
    "01001... 10001011 11111100",
    "10111001 01001100 00000000 00000000 00000000",
    "10111000 11001100 11001100 11001100 11001100",
    "11110011 10101011",
    "01001... 10001011 10001100 ..100100 01000000 00000001 00000000 00000000",
    "01001... 10001011 10000100 ..100100 01000000 00000001 00000000 00000000",
);

// 1420fbda7 48 8b 44        MOV        RAX,qword ptr [RSP + 0x28]=>local_50
//           24 28
// 1420fbdac 8b 40 04        MOV        EAX,dword ptr [RAX + 0x4]
// 1420fbdaf c1 e8 10        SHR        EAX,0x10
// 1420fbdb2 83 f8 05        CMP        EAX,0x5
// 1420fbdb5 74 07           JZ         LAB_1420fbdbe
// 1420fbdb7 33 c0           XOR        EAX,EAX
// 1420fbdb9 e9 59 01        JMP        LAB_1420fbf17
//           00 00
// 1420fbdbe e8 cd bb        CALL       FUN_1420b7990
//           fb ff
pub(crate) const GET_ALLOCATOR_PATTERN: &str = concat!(
    "01001... 10001011 01000100 ..100100 00101000",
    "10001011 01000000 00000100",
    "11000001 11101000 00010000",
    "10000011 11111000 00000101",
    "01110100 ........",
    "00110011 11000000",
    "11101001 ........ ........ ........ ........",
    "11101000 [........ ........ ........ ........]",
);
