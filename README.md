# Rust-eze (LESS Rust)
> LESS parser (goal: compiler) written in Rust. Highly work in progress.

## Tokenizer
The tokenizer consumes a stream of Unicode codepoints (Rust: `char`) and produces a stream of tokens. As it stands this process is directly based on the [tokenizer algorithms](https://www.w3.org/TR/css-syntax-3/#tokenizer-algorithms) in the official CSS specification.

## Parser
The parser consumes a stream of tokens and produces an Abstract Syntax Tree (AST) of the LESS stylesheet. This process is based on the [parser algorithms](https://www.w3.org/TR/css-syntax-3/#parser-algorithms) in the official CSS specification, however since LESS is a superset of CSS there are some changes.

#### Consume a LESS at-rule
Consume the next input token (expected to be **\<at-keyword-token>**) and remember its value as *name*.

While the next input token is whitespace, consume the next input token.

Consume the next input token:

- **\<colon-token>**
  - Consume a LESS variable declaration with *name*, and return it.
- **\<(-token>** and the next input token is a **\<)-token>**
  - Consume a LESS variable call with *name*, and return it.
- **\<[-token>**
  - Consume a LESS namespace lookup with *value* set to **VariableCall { *name* }**, and return it. (Note: the block might have arguments, which would render it an invalid rule, but we don't handle this here.)
- **\<;-token>**
  - This is a parse error, return nothing. (Note: this is only a parse error for LESS stylesheets, since CSS does allow empty at-rules.)
- **anything else**
  - Reconsume the current input token. Consume a CSS at-rule with *name*, and return it.

#### Consume a LESS qualified rule
TODO
- Consume *prelude* (= selectors or mixin name)
- If prelude is a valid mixin name, and is followed by **\<(-token>** -> consume a mixin call/declaration with *prelude*?
- Otherwise, consume a CSS qualified rule with *prelude*

#### Consume a LESS variable declaration
TODO
- Repeatedly consume component values (or expressions?) into list *value*.
- Return **VariableDeclaration { *value* }**

#### Consume a LESS variable call
TODO:
- Consume a **\<)-token>** (since a variable call cannot have arguments). Create *call* set to **VariableCall { *name* }**.
- If next input token is **\<[-token>**. Consume a LESS namespace lookup with *value* set to *call*, and return it.
- Otherwise. Return *call*.

#### Consume a LESS namespace lookup
TODO: 
- Repeatedly consume lookup into list *lookups* 
- Return **NamespaceValue { *value*, *lookups* }**

#### Consume a mixin call/declaration
TODO: Called with a *prelude*.
1. Consume a **\<(-token>**
2. Consume tokens into a list *arguments* until a **\<)-token>**
3. Consume all whitespace.
4. Consume the next token:
- **\<[-token>**
  - Consume a LESS namespace lookup with *value* set to **MixinCall { *prelude*, *arguments* }**, return it.
- **\<;-token>**
- ...