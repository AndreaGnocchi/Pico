# Pico

Dynamically typed, integer-focused, with functions and recursion but no closures, which keeps the VM simple.

---
## Keywords

`let` `fn` `if` `else` `while` `return` `print` `true` `false` `nil`

## Values
- 64-bit ints, bools, `nil`

## Syntax

```
// comment
let x = 10;
x = x + 1;

fn fib(n) {
    if n < 2 { return n; }
    return fib(n - 1) + fib(n - 2);
}

let i = 0;
while i < 10 {
    print fib(i);
    i = i + 1;
}
```

## Grammar (EBNF)

```
program   = { stmt } ;
stmt      = "let" IDENT "=" expr ";"
          | IDENT "=" expr ";"
          | "print" expr ";"
          | "return" [ expr ] ";"
          | "if" expr block [ "else" ( block | ifstmt ) ]
          | "while" expr block
          | "fn" IDENT "(" [ IDENT { "," IDENT } ] ")" block
          | expr ";" ;
block     = "{" { stmt } "}" ;
expr      = or ;
or        = and { "||" and } ;
and       = equality { "&&" equality } ;
equality  = compare { ("==" | "!=") compare } ;
compare   = term { ("<" | "<=" | ">" | ">=") term } ;
term      = factor { ("+" | "-") factor } ;
factor    = unary { ("*" | "/" | "%") unary } ;
unary     = ("-" | "!") unary | call ;
call      = primary { "(" [ expr { "," expr } ] ")" } ;
primary   = INT | "true" | "false" | "nil" | IDENT | "(" expr ")" ;
```

Truthiness: `false` and `nil` are falsy, everything else is truthy.
