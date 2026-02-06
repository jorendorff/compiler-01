use std::collections::HashMap;
use std::fmt::Write;

use crate::ast::{BinOp, Expr, Stmt};

pub struct Codegen {
    output: String,
    /// Maps variable names to their offset from the frame pointer (x29).
    /// Offsets are negative (variables are below the frame pointer).
    variables: HashMap<String, i64>,
    /// Next available stack offset for a variable (grows downward).
    next_var_offset: i64,
    /// Total number of variable slots allocated (used to size the stack frame).
    var_count: usize,
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {
            output: String::new(),
            variables: HashMap::new(),
            next_var_offset: -8, // First variable at [x29, #-8]
            var_count: 0,
        }
    }

    /// Count the total number of variable slots needed by the program.
    /// Each `let` statement allocates a new slot (even if shadowing).
    fn count_variables(stmts: &[Stmt]) -> usize {
        let mut count = 0;
        for stmt in stmts {
            if matches!(stmt, Stmt::Let { .. }) {
                count += 1;
            }
        }
        count
    }

    pub fn generate(mut self, stmts: &[Stmt]) -> Result<String, String> {
        self.var_count = Self::count_variables(stmts);

        // Calculate stack frame size:
        // - 16 bytes for saved x29 (frame pointer) and x30 (link register)
        // - 8 bytes per variable
        // - Round up to 16-byte alignment
        let vars_size = (self.var_count as i64) * 8;
        let frame_size = 16 + vars_size;
        let frame_size = (frame_size + 15) & !15; // align to 16

        // Data section
        writeln!(self.output, ".section __DATA,__data").unwrap();
        writeln!(self.output, "_fmt:").unwrap();
        writeln!(self.output, "    .asciz \"%lld\\n\"").unwrap();
        writeln!(self.output).unwrap();

        // Text section
        writeln!(self.output, ".section __TEXT,__text").unwrap();
        writeln!(self.output, ".globl _main").unwrap();
        writeln!(self.output, ".p2align 2").unwrap();
        writeln!(self.output, "_main:").unwrap();

        // Prologue: allocate stack frame, save frame pointer and link register
        // Frame layout (high to low):
        //   [x29+8]  = saved x30 (link register)
        //   [x29]    = saved x29 (frame pointer)
        //   [x29-8]  = variable 0
        //   [x29-16] = variable 1
        //   ...
        //   [sp]     = bottom of frame
        writeln!(self.output, "    sub sp, sp, #{frame_size}").unwrap();
        writeln!(self.output, "    stp x29, x30, [sp, #{}]", frame_size - 16).unwrap();
        writeln!(self.output, "    add x29, sp, #{}", frame_size - 16).unwrap();

        // Generate code for each statement
        for stmt in stmts {
            self.gen_stmt(stmt)?;
        }

        // Epilogue: return 0
        writeln!(self.output, "    mov x0, #0").unwrap();
        writeln!(self.output, "    ldp x29, x30, [sp, #{}]", frame_size - 16).unwrap();
        writeln!(self.output, "    add sp, sp, #{frame_size}").unwrap();
        writeln!(self.output, "    ret").unwrap();

        Ok(self.output)
    }

    fn gen_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Let { name, expr } => {
                // Evaluate the expression (result in x0).
                // Important: evaluate BEFORE allocating the new slot,
                // so that `let x = x + 1;` reads the old x.
                self.gen_expr(expr)?;
                // Allocate a new variable slot
                let offset = self.next_var_offset;
                self.next_var_offset -= 8;
                self.variables.insert(name.clone(), offset);
                // Store the value
                writeln!(self.output, "    str x0, [x29, #{}]", offset).unwrap();
                Ok(())
            }
            Stmt::Assign { name, expr } => {
                let offset = *self.variables.get(name).ok_or_else(|| {
                    format!("undefined variable '{}'", name)
                })?;
                self.gen_expr(expr)?;
                writeln!(self.output, "    str x0, [x29, #{}]", offset).unwrap();
                Ok(())
            }
            Stmt::Print { expr } => {
                self.gen_expr(expr)?;
                // On ARM64 macOS, variadic arguments to printf are passed on
                // the stack, not in registers. The format string (named param)
                // goes in x0. The variadic i64 value goes at [sp].
                // We need to allocate stack space for the variadic arg.
                writeln!(self.output, "    str x0, [sp, #-16]!").unwrap();
                // Load format string address into x0 (first arg).
                self.gen_load_address("x0", "_fmt");
                // Call printf
                writeln!(self.output, "    bl _printf").unwrap();
                // Restore stack
                writeln!(self.output, "    add sp, sp, #16").unwrap();
                Ok(())
            }
        }
    }

    fn gen_load_address(&mut self, reg: &str, label: &str) {
        // Use adrp + add to form a PC-relative address (required on macOS ARM64)
        writeln!(self.output, "    adrp {reg}, {label}@PAGE").unwrap();
        writeln!(self.output, "    add {reg}, {reg}, {label}@PAGEOFF").unwrap();
    }

    fn gen_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::IntLit(val) => {
                self.gen_load_immediate(*val);
                Ok(())
            }
            Expr::Var(name) => {
                let offset = *self.variables.get(name).ok_or_else(|| {
                    format!("undefined variable '{}'", name)
                })?;
                writeln!(self.output, "    ldr x0, [x29, #{}]", offset).unwrap();
                Ok(())
            }
            Expr::UnaryMinus(inner) => {
                self.gen_expr(inner)?;
                writeln!(self.output, "    neg x0, x0").unwrap();
                Ok(())
            }
            Expr::BinOp { op, left, right } => {
                // Evaluate left side, result in x0
                self.gen_expr(left)?;
                // Push x0 onto the stack (save left result)
                writeln!(self.output, "    str x0, [sp, #-16]!").unwrap();
                // Evaluate right side, result in x0
                self.gen_expr(right)?;
                // Pop left result into x1
                writeln!(self.output, "    ldr x1, [sp], #16").unwrap();
                // Now: x1 = left, x0 = right
                // Compute result into x0
                match op {
                    BinOp::Add => {
                        writeln!(self.output, "    add x0, x1, x0").unwrap();
                    }
                    BinOp::Sub => {
                        writeln!(self.output, "    sub x0, x1, x0").unwrap();
                    }
                    BinOp::Mul => {
                        writeln!(self.output, "    mul x0, x1, x0").unwrap();
                    }
                    BinOp::Div => {
                        writeln!(self.output, "    sdiv x0, x1, x0").unwrap();
                    }
                    BinOp::Mod => {
                        // ARM64 has no remainder instruction.
                        // a % b = a - (a / b) * b
                        // x1 = left (a), x0 = right (b)
                        writeln!(self.output, "    sdiv x2, x1, x0").unwrap();
                        writeln!(self.output, "    msub x0, x2, x0, x1").unwrap();
                    }
                }
                Ok(())
            }
        }
    }

    fn gen_load_immediate(&mut self, val: i64) {
        if val >= 0 && val < 65536 {
            writeln!(self.output, "    mov x0, #{}", val).unwrap();
        } else if val < 0 && val >= -65536 {
            // movn loads the bitwise NOT of the shifted immediate.
            // To load a negative value v, we use movn with the NOT of v.
            let not_val = !val as u64;
            writeln!(self.output, "    movn x0, #{}", not_val & 0xFFFF).unwrap();
        } else {
            // For arbitrary 64-bit values, use movz + movk sequence.
            let uval = val as u64;
            writeln!(self.output, "    movz x0, #{}", uval & 0xFFFF).unwrap();
            if (uval >> 16) & 0xFFFF != 0 {
                writeln!(
                    self.output,
                    "    movk x0, #{}, lsl #16",
                    (uval >> 16) & 0xFFFF
                )
                .unwrap();
            }
            if (uval >> 32) & 0xFFFF != 0 {
                writeln!(
                    self.output,
                    "    movk x0, #{}, lsl #32",
                    (uval >> 32) & 0xFFFF
                )
                .unwrap();
            }
            if (uval >> 48) & 0xFFFF != 0 {
                writeln!(
                    self.output,
                    "    movk x0, #{}, lsl #48",
                    (uval >> 48) & 0xFFFF
                )
                .unwrap();
            }
        }
    }
}
