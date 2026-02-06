# The Toy Language

Toy is a small imperative programming language that supports integer arithmetic,
variables, and printing.

## Building the compiler

Requires a Rust toolchain (edition 2024). From the project root:

```sh
cargo build --release
```

The compiler binary is at `target/release/toy-compiler`.

## Running the compiler

```sh
toy-compiler <input.toy> [-o <output>]
```

- `<input.toy>` — path to a Toy source file.
- `-o <output>` — (optional) path for the output executable. Defaults to the
  input filename without its extension.

The compiler produces a native executable for the current platform
(aarch64-apple-darwin). It requires `as` (the system assembler) and `cc`
(the system C compiler/linker) to be available in `PATH`.

### Example

```sh
echo 'print 6 * 7;' > hello.toy
toy-compiler hello.toy -o hello
./hello
# Output: 42
```

## Language reference

### Overview

A Toy program is a sequence of statements, executed from top to bottom. The
program exits with code 0 after the last statement.

All values are signed 64-bit integers.

### Lexical structure

The lexer uses **maximal munch**: at each point, it consumes the longest
possible token. Whitespace and comments separate tokens but are not themselves
tokens.

This has practical consequences:

- Symbols (`+`, `-`, `*`, etc.) always act as token boundaries. No whitespace
  is needed around them. `3+4` is three tokens: `3`, `+`, `4`.

- Keywords and identifiers are both "words" (sequences of letters, digits, and
  underscores). A keyword is only recognized when the full word matches. If
  extra alphanumeric characters are attached, the whole run is lexed as an
  identifier:

  - `print 3;` — keyword `print`, integer `3`, semicolon. OK.
  - `print(3);` — keyword `print`, `(`, `3`, `)`, `;`. OK (the `(` terminates
    the keyword).
  - `print3;` — identifier `print3`, `;`. Not a print statement. This is a
    syntax error (identifiers can only begin a statement if followed by `=`).
  - `let x=1;` — keyword `let`, identifier `x`, `=`, `1`, `;`. OK.
  - `letx=1;` — identifier `letx`, `=`, `1`, `;`. Not a let statement; this
    is an assignment to a variable called `letx`.

- Integer literals cannot be immediately followed by letters. However, the
  current lexer does not enforce this; `3x` is lexed as integer `3` followed
  by identifier `x`. This is a consequence of maximal munch (digits stop at
  the first non-digit, then a new identifier token begins). Whether this
  produces a valid program depends on context; in most cases it is a syntax
  error.

**Whitespace:** Spaces, tabs, and newlines are insignificant (they separate
tokens but are otherwise ignored).

**Comments:** Line comments start with `//` and extend to the end of the line.

```
// This is a comment.
print 42; // This is also a comment.
```

**Identifiers:** A letter or underscore, followed by zero or more letters,
digits, or underscores. Letters are ASCII only (`a`–`z`, `A`–`Z`).

```
x
foo_bar
_temp
myVar2
```

**Keywords:** `let` and `print` are reserved and cannot be used as variable
names.

**Integer literals:** A sequence of one or more decimal digits (`0`–`9`). There
is no negative literal syntax; use the unary minus operator instead. Integer
literals must be in the range 0 to 9223372036854775807 (2^63 − 1, i.e.
`i64::MAX`).

```
0
42
9223372036854775807
```

**Symbols:** `+`, `-`, `*`, `/`, `%`, `=`, `;`, `(`, `)`.

### Grammar

```
program     = statement*
statement   = let_stmt | assign_stmt | print_stmt
let_stmt    = "let" IDENT "=" expr ";"
assign_stmt = IDENT "=" expr ";"
print_stmt  = "print" expr ";"

expr        = term (("+" | "-") term)*
term        = unary (("*" | "/" | "%") unary)*
unary       = "-" unary | atom
atom        = INT_LITERAL | IDENT | "(" expr ")"
```

### Statements

#### `let` — variable declaration

```
let x = 10;
```

Declares a new variable and initializes it with the value of the expression.
The expression is evaluated before the variable is created, so a `let`
statement can refer to a previously declared variable of the same name:

```
let x = 1;
let x = x + 1;  // x is now 2
print x;         // prints 2
```

This is called **shadowing**: the new `x` shadows the old `x`. The old
variable is no longer accessible.

#### Assignment

```
x = x + 1;
```

Assigns a new value to an existing variable. The variable must have been
previously declared with `let`. Assigning to an undeclared variable is a
compile error.

#### `print`

```
print 42;
print x * 2 + 1;
```

Evaluates the expression and prints its value as a decimal integer, followed
by a newline.

### Expressions

#### Integer literals

Decimal integer constants in the range 0 to 9223372036854775807.

#### Variable references

An identifier that was previously declared with `let`.

#### Parenthesized expressions

```
(expr)
```

Parentheses override the default precedence.

#### Unary minus

```
-expr
```

Negates the value. Unary minus has higher precedence than all binary operators.

#### Binary operators

From highest to lowest precedence:

| Precedence | Operators    | Description                        |
| ---------- | ------------ | ---------------------------------- |
| 1 (high)   | `*`, `/`, `%`| Multiplication, division, modulo   |
| 2 (low)    | `+`, `-`     | Addition, subtraction              |

All binary operators are **left-associative**:

```
print 10 - 3 - 2;  // (10 - 3) - 2 = 5
print 24 / 4 / 2;  // (24 / 4) / 2 = 3
```

### Arithmetic semantics

All arithmetic operates on signed 64-bit integers (range: −2^63 to 2^63 − 1).

- **Addition, subtraction, multiplication:** On overflow, the result wraps
  around (two's complement). For example,
  `9223372036854775807 + 1 = -9223372036854775808`.

- **Negation:** Unary minus wraps on overflow. The only overflowing case is
  negating the minimum value: if `x` is −2^63, then `-x` wraps back to −2^63.

- **Division (`/`):** Truncates toward zero.
  `7 / 2 = 3`, `-7 / 2 = -3`.
  The one overflowing case, −2^63 / −1, wraps: the result is −2^63.

- **Modulo (`%`):** The result has the same sign as the dividend (left operand).
  `7 % 3 = 1`, `-7 % 3 = -1`.

- **Division or modulo by zero:** The program crashes (the ARM64 `sdiv`
  instruction triggers a hardware trap).

### Limits

- A program may contain at most 32 `let` statements (including shadowing
  re-declarations).

- Expressions may be nested to a depth of at most 256 (counting parenthesized
  sub-expressions and chained unary minus operators).

Both are compile-time limits; the compiler reports an error if either is
exceeded.

### Error handling

The compiler reports errors and exits with a nonzero status for:

- Lexical errors (unexpected characters)
- Syntax errors (malformed statements or expressions)
- Undefined variables (use before `let`, or assignment to undeclared variable)
- Integer literals out of range
- Too many variables (more than 32 `let` statements)
- Expression nesting too deep (more than 256 levels)
