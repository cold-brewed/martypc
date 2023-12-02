/*
    MartyPC
    https://github.com/dbalsom/martypc

    Copyright 2022-2023 Daniel Balsom

    Permission is hereby granted, free of charge, to any person obtaining a
    copy of this software and associated documentation files (the “Software”),
    to deal in the Software without restriction, including without limitation
    the rights to use, copy, modify, merge, publish, distribute, sublicense,
    and/or sell copies of the Software, and to permit persons to whom the
    Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
    FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

    ---------------------------------------------------------------------------

    cpu_808x::stack.rs

    Implements stack-oriented routines such as push and pop.

*/

use crate::cpu_808x::{biu::*, *};

impl Cpu {
    pub fn push_u8(&mut self, data: u8, flag: ReadWriteFlag) {
        // Stack pointer grows downwards
        self.sp = self.sp.wrapping_sub(2);
        self.biu_write_u8(Segment::SS, self.sp, data, flag);
    }

    pub fn push_u16(&mut self, data: u16, flag: ReadWriteFlag) {
        // Stack pointer grows downwards
        self.sp = self.sp.wrapping_sub(2);
        self.biu_write_u16(Segment::SS, self.sp, data, flag);
    }

    pub fn pop_u16(&mut self) -> u16 {
        let result = self.biu_read_u16(Segment::SS, self.sp, ReadWriteFlag::Normal);

        // Stack pointer shrinks upwards
        self.sp = self.sp.wrapping_add(2);
        result
    }

    pub fn push_register16(&mut self, reg: Register16, flag: ReadWriteFlag) {
        // Stack pointer grows downwards
        self.sp = self.sp.wrapping_sub(2);

        let data = match reg {
            Register16::AX => self.ax,
            Register16::BX => self.bx,
            Register16::CX => self.cx,
            Register16::DX => self.dx,
            Register16::SP => self.sp,
            Register16::BP => self.bp,
            Register16::SI => self.si,
            Register16::DI => self.di,
            Register16::CS => self.cs,
            Register16::DS => self.ds,
            Register16::SS => self.ss,
            Register16::ES => self.es,
            Register16::IP => self.ip,
            _ => panic!("Invalid register"),
        };

        self.biu_write_u16(Segment::SS, self.sp, data, flag);
    }

    pub fn pop_register16(&mut self, reg: Register16, flag: ReadWriteFlag) {
        let data = self.biu_read_u16(Segment::SS, self.sp, flag);

        let mut update_sp = true;
        match reg {
            Register16::AX => self.set_register16(reg, data),
            Register16::BX => self.set_register16(reg, data),
            Register16::CX => self.set_register16(reg, data),
            Register16::DX => self.set_register16(reg, data),
            Register16::SP => {
                self.sp = data;
                update_sp = false;
            }
            Register16::BP => self.bp = data,
            Register16::SI => self.si = data,
            Register16::DI => self.di = data,
            Register16::CS => {
                self.biu_update_cs(data);
            }
            Register16::DS => self.ds = data,
            Register16::SS => {
                self.ss = data;
                // Inhibit interrupts for one instruction after issuing POP SS
                self.interrupt_inhibit = true
            }
            Register16::ES => self.es = data,
            Register16::IP => self.ip = data,
            _ => panic!("Invalid register"),
        };
        // Stack pointer grows downwards
        if update_sp {
            self.sp = self.sp.wrapping_add(2);
        }
    }

    pub fn push_flags(&mut self, wflag: ReadWriteFlag) {
        // Stack pointer grows downwards
        self.sp = self.sp.wrapping_sub(2);
        self.biu_write_u16(Segment::SS, self.sp, self.flags, wflag);
    }

    pub fn pop_flags(&mut self) {
        let result = self.biu_read_u16(Segment::SS, self.sp, ReadWriteFlag::Normal);

        let trap_was_set = self.get_flag(Flag::Trap);

        // Ensure state of reserved flag bits
        self.flags = result & FLAGS_POP_MASK;
        self.flags |= CPU_FLAGS_RESERVED_ON;

        // Was trap flag just set? Set trap enable delay.
        let trap_is_set = self.get_flag(Flag::Trap);
        if !trap_was_set && trap_is_set {
            self.trap_enable_delay = 2;
        }

        // Was trap flag just disabled? Set trap disable delay.
        if trap_was_set && !trap_is_set {
            self.trap_disable_delay = 1;
        }

        // Stack pointer grows downwards
        self.sp = self.sp.wrapping_add(2);
    }

    pub fn release(&mut self, disp: u16) {
        // TODO: Stack exceptions?
        self.sp = self.sp.wrapping_add(disp);
    }
}
