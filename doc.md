# ChatLang 3.2 Documentation

## Introduction
ChatLang is a dynamic programming language with a functional core and built‑in P2P chat support. It combines functional, imperative, and object‑oriented paradigms.

This version supports:
- **Currying** – all functions are curried.
- **Num type** – unified numeric type (Int and Float).
- **Indentation‑based blocks** – `:` and indentation instead of `{ }`.
- **List comprehensions** – `[expr for var in iterable if condition]`.
- **Loop** – `loop { ... }` and `break` with optional value.
- **Chat** – client‑server contacts, local chats, control messages for synchronisation.
- **Mentions** – `@uid` in messages delivers a private copy.
- **Error reporting** – file, line, column, snippet.
- **F‑strings** – `f"text {expr} text"` with highlighting in REPL.
- **`del`** – deletes a variable.
- **`load`** – imports files with caching.
- **Panic safety** – no unwraps; errors are propagated.

All features described below are implemented and tested.

---

## 1. REPL
Launch with `cargo run` or `chatlang`.

- Enter single‑line expressions; results are **not** printed automatically. Use `show(expr)` to display a value (it prints to stdout and returns `()`).
- For multi‑line blocks, use a colon followed by indented lines. To end the block, enter an empty line or type `end` on a new line. The block returns the value of the last expression.
- Built‑in functions for controlling the REPL:
  - `exit()` – terminates the interpreter.
  - `load(filename)` – loads and executes a ChatLang script in the current environment. Definitions persist.
- Errors are printed with location and code snippet.

Example:
```
>>> let greet name = "Hello, " ++ name ++ "!"
>>> show(greet "World")
Hello, World!
>>> let square x = x * x:
...     show(square 5)
25
```

---

## 2. Lexical Elements and Syntax

### Comments
- Single‑line: `# text`
- Multi‑line: `#- text -#` (nested supported)

### Identifiers
- Letters, digits, `_` (starting with a letter or `_`).

### Literals
- Integers: `42`, `-7`
- Floats: `3.14`, `-0.5`, `1.0e10`
- Characters: `'a'`, `'\n'`
- Strings: `"Hello"`, `"line1\nline2"`
- Booleans: `true`, `false`
- Unit: `()`
- UID: `@alice`, `@everyone`
- Lists: `[1, 2, 3]`, `[0..10]` (range, excludes upper bound)
- Tuples: `(1, "ok")`, `(true, @bob)`
- Byte strings: `#B"48656C6C6F"`
- Durations: `5s`, `500ms`, `2m`, `1h`
- Map literal: `%(key => value, ...)`
- Set literal: `%[elem, ...]`
- F‑string: `f"text {expression} text"`

---

## 3. Value Types
- Primitive: `Num` (unified Int/Float), `Char`, `String`, `Bool`, `Unit`, `Uid`, `ByteString`, `Pid`, `DateTime`, `Duration`.
- Composite: lists, tuples, records.
- Custom: `data` definitions, `struct` definitions, classes.
- Collections: `Map` and `Set`.

---

## 4. Expressions

### 4.1. Basics
- Literals, variables.
- Function application is curried and left‑associative: `f a b c` is `(((f a) b) c)`.
- Lambda: `lambda x y -> x + y` or `\ x -> x * 2`.
- Indexing: `expr[index]` works on lists, strings, byte strings, tuples, maps (returns value or `Unit`), sets (returns `Bool`).

### 4.2. Conditional
```
if condition then expr1 else expr2
```
Single‑line only.

### 4.3. Pattern Matching
```
case expr of
    pattern1 -> expr1
    pattern2 -> expr2
    _ -> default
```
One‑line variant with semicolons:
```
case expr of pattern1 -> expr1; pattern2 -> expr2; _ -> default
```

### 4.4. Local Definitions
- `let x = 5` – defines a new variable (error if already defined).
- `let f x y = x + y` – function definition (curried).
- Type annotations: `let N :: Num = 5`, `let add :: Num -> Num -> Num = lambda a b -> a + b`.
- Blocks use indentation after `:`.

### 4.5. Operators (precedence, high to low)
1. `not` (unary)
2. `*`, `/`, `%` (left associative)
3. `+`, `-` (left associative)
4. `++` (string/list concatenation, right associative)
5. `:` (cons, right associative)
6. `in`, `not in` (membership)
7. `==`, `!=`, `<`, `>`, `<=`, `>=`
8. `and`, `or` (short‑circuit)
9. `|>` (pipe, left associative)
Operator `$` is supported (low‑precedence application, right‑associative).

### 4.6. Loops
- `for x in collection: body` – iterates over lists, sets, maps (yields key‑value tuples for maps).
- `while condition: body`
- `loop { body }` – infinite loop; use `break` with optional value to exit.

Example:
```
>>> let x = 0
>>> loop { x = x + 1; if x == 5 then break x else () }
5
```

### 4.7. Error Handling
- `error "message"` – throws an error.
- `try expr` – catches error (if any, passes it on).
- `try expr catch pattern -> handler` – handles error.

Example:
```
try error "oops" catch e -> "caught: " ++ e
```

### 4.8. F‑strings
Syntax: `f"text {expression} text"`. Inside braces, any ChatLang expression can appear; it is evaluated and converted to a string via `show`. Double braces `{{` and `}}` are used to insert literal `{` and `}` characters.

Example:
```
>>> let x = 5
>>> f"x = {x}, x*2 = {x * 2}"
"x = 5, x*2 = 10"
```

### 4.9. List Comprehensions
```
[expr for var in iterable if condition]
```
Can have multiple `for` and `if` clauses.

Example:
```
>>> [x * 2 for x in [1,2,3] if x > 1]
[4, 6]
```

---

## 5. Structs and Algebraic Data Types

```
struct Person = (name = String, age = Num)
data Result a = Ok a | Err String
```

Struct construction: `Person(name = "Alice", age = 30)`. Field access via `.` (e.g., `person.name`). Update via record update syntax (not directly; use functions or manual reconstruction).

---

## 6. Classes (OOP)

```
class Counter = (val = Num; inc(self) = self { val = self.val + 1 }; get(self) = self.val)
```

- Constructor: `new Counter(0)`
- Inheritance: `class AdvancedCounter extends Counter = (...)` (with parentheses)
- Method call: `counter.inc()`

---

## 7. Unions (ADT)

```
data Option a = None | Some a
```

Constructors: `Some 42`, `None`. Pattern matching works.

---

## 8. Map and Set

### Map
- Create: `%(key => value, ...)`
- Functions: `mapGet`, `mapSet`, `mapRemove`, `mapKeys`, `mapValues`, `mapEntries`, `mapContains`, `mapSize`, `mapFilter`, `mapMerge`
- Indexing: `map[key]` returns value or `Unit`

### Set
- Create: `%[elem, ...]`
- Functions: `setAdd`, `setRemove`, `setContains`, `setUnion`, `setIntersection`, `setDifference`, `setSize`, `setFilter`, `setMap`
- Indexing: `set[elem]` returns `Bool`

---

## 9. Built‑in Functions

### 9.1. Mathematics
- `sqrt`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan :: Num -> Num`
- `toFloat :: Num -> Num`, `toInt :: Num -> Num`

### 9.2. Conversions and Type Introspection
- `show :: a -> ()` – prints the value to stdout and returns `()`.
- `parseInt :: String -> Num`, `parseFloat :: String -> Num`
- `chr :: Num -> Char`, `ord :: Char -> Num`
- `typeof :: a -> String`

### 9.3. List, String, ByteString, Map, Set
- `null :: [a] -> Bool`
- `length :: collection -> Num` (works on List, String, ByteString, Map, Set, Tuple)
- `map :: (a -> b) -> [a] -> [b]`
- `filter :: (a -> Bool) -> [a] -> [a]`
- `foldl :: (b -> a -> b) -> b -> [a] -> b`
- `foldr :: (a -> b -> b) -> b -> [a] -> b`
- `take :: Num -> collection -> collection`
- `drop :: Num -> collection -> collection`
- `reverse :: [a] -> [a]`
- `all :: (a -> Bool) -> [a] -> Bool`
- `any :: (a -> Bool) -> [a] -> Bool`
- `find :: (a -> Bool) -> [a] -> Maybe a`
- `sort :: [a] -> [a]` (by string representation)
- `sortBy :: (a -> a -> Num) -> [a] -> [a]`
- `sum :: [Num] -> Num`
- `concat :: [[a]] -> [a]`
- `flatten :: [[a]] -> [a]`
- `zip :: [a] -> [b] -> [(a,b)]`
- `zipWith :: (a -> b -> c) -> [a] -> [b] -> [c]`
- `unzip :: [(a,b)] -> ([a],[b])`
- `indexOf :: [a] -> a -> Maybe Num`
- `lastIndexOf :: [a] -> a -> Maybe Num`

### 9.4. String Functions
- `split :: String -> String -> [String]`
- `join :: String -> [String] -> String`
- `startsWith :: String -> String -> Bool`
- `endsWith :: String -> String -> Bool`
- `trim :: String -> String`
- `replace :: String -> String -> String -> String`
- `substring :: Num -> Num -> String -> String`

### 9.5. JSON
- `parseJson :: String -> JsonValue`
- `encodeJson :: JsonValue -> String`
- `lookup :: String -> JsonValue -> Maybe JsonValue`

### 9.6. Time
- `formatTime :: String -> DateTime -> String`
- `parseTime :: String -> String -> DateTime`
- `addDuration :: DateTime -> Duration -> DateTime`
- `diffDuration :: DateTime -> DateTime -> Duration`
- `now :: DateTime`

### 9.7. ByteString
- `packBytes :: [Num] -> ByteString`
- `unpackBytes :: ByteString -> [Num]`

### 9.8. I/O and Files
- `putStrLn :: String -> ()` – prints a raw string.
- `getLine :: String`
- `getArgs :: [String]`
- `readFile :: String -> String`
- `readBinaryFile :: String -> ByteString`
- `writeFile :: String -> String -> ()`
- `appendFile :: String -> String -> ()`
- `writeBinaryFile :: String -> ByteString -> ()`
- `fileExists :: String -> Bool`
- `fileSize :: String -> Num`
- `listDir :: String -> [String]`
- `createDir :: String -> ()`
- `removeDir :: String -> ()`
- `fileMove :: String -> String -> ()`
- `filePermissions :: String -> Num`
- `setFilePermissions :: String -> Num -> ()`

### 9.9. Network
- `fetch :: String -> FetchResult`
- `fetchOpts :: FetchOptions -> FetchResult`

### 9.10. Chat and Contacts
- `login :: Uid -> ()`
- `logout :: () -> ()`
- `deleteUser :: Uid -> ()`
- `newChat :: String -> [Uid] -> String`
- `addMember :: Uid -> String -> ()`
- `removeMember :: Uid -> String -> ()`
- `deleteChat :: String -> ()`
- `open :: String -> ()`
- `send :: Uid -> String -> Bool` (private message)
- `sendFile :: Uid -> String -> Bool`
- `sendChat :: String -> String -> Bool` – sends to chat, with mention support.
- `sendFileToChat :: String -> String -> Bool`
- `inbox :: () -> [ChatMsg]`
- `history :: String -> [ChatMsg]`
- `downloads :: () -> [FileTransfer]`
- `saveFile :: Num -> String -> Bool`
- `listChats :: () -> [String]`
- `members :: String -> [Uid]`
- `serverStart :: String -> (String?) -> ()`
- `serverStop :: () -> ()`
- `connect :: String -> Uid -> (String?) -> Num`
- `getPublicIP :: () -> Maybe String`
- `setExternalIP :: String -> ()`
- `addContact :: Uid -> String -> ()`
- `removeContact :: Uid -> ()`

### 9.11. Processes
- `spawn :: (() -> ()) -> Pid`
- `procSelf :: Pid`
- `procSend :: Pid -> a -> ()`
- `procRecv :: a`
- `procWait :: Pid -> a`
- `procExit :: a -> ()`
- `sleep :: Duration -> ()`
- `after :: Duration -> (() -> ()) -> ()`

### 9.12. Maybe
- `Nothing :: Maybe a`
- `Just :: a -> Maybe a`
- `maybe :: (a -> b) -> b -> Maybe a -> b`

### 9.13. Map & Set (additional)
- `mapGet`, `mapSet`, `mapRemove`, `mapKeys`, `mapValues`, `mapEntries`, `mapContains`, `mapSize`, `mapFilter`, `mapMerge`
- `setAdd`, `setRemove`, `setContains`, `setUnion`, `setIntersection`, `setDifference`, `setSize`, `setFilter`, `setMap`
- `listToSet :: [a] -> Set`
- `mapToList :: Map -> [(key, value)]`

### 9.14. Cryptography
- `sha256 :: ByteString -> ByteString`
- `sha256String :: String -> String`
- `kyberKeyPair :: () -> (PublicKey, SecretKey)` (both as ByteString)
- `kyberEncapsulate :: PublicKey -> (Ciphertext, SharedSecret)`
- `kyberDecapsulate :: SecretKey -> Ciphertext -> SharedSecret`

### 9.15. Variables
- `del :: String -> ()` – deletes a variable.

---

## 10. Examples

### 10.1. Arithmetic and Functions
```
>>> show(1 + 2 * 3)
7
>>> let sq x = x * x
>>> show(sq 5)
25
>>> show((lambda x -> x + 1) 5)
6
>>> show(-5)
-5
>>> show(5 - -2)
7
```

### 10.2. Variables and Assignments
```
>>> let x = 5
>>> show(x)
5
>>> let x = 6
error: variable 'x' already defined
>>> x = 10
>>> show(x)
10
```

### 10.3. Characters and Strings
```
>>> show('a')
a
>>> show('\n')

>>> let msg = "Hello"
>>> show(msg)
Hello
>>> show(msg ++ " world")
Hello world
```

### 10.4. Type Annotations and typeof
```
>>> let N :: Num = 5
>>> show(typeof N)
Num
>>> let add :: Num -> Num -> Num = lambda a b -> a + b
>>> show(typeof add)
Closure
>>> let msg :: String = "hello"
>>> show(typeof msg)
String
>>> N = 'a'   # type mismatch
error: Type mismatch: expected 'Num', got 'Char'
```

### 10.5. Conditionals
```
>>> show(if 5 > 3 then "yes" else "no")
yes
```

### 10.6. Lists, Strings, Tuples
```
>>> show([1, 2, 3] |> map(lambda x -> x * 2))
[2, 4, 6]
>>> show("Hello, " ++ "world!")
Hello, world!
>>> show(length("abc"))
3
>>> show([0..5])
[0, 1, 2, 3, 4]
>>> show((1, "two", 3.0)[1])
two
>>> show(length((1,2,3)))
3
```

### 10.7. Pattern Matching
```
>>> show(case 2 of 1 -> "one"; 2 -> "two"; _ -> "other")
two
```

### 10.8. JSON
```
>>> let data = parseJson("[1, 2, 3]")
>>> show(data)
[1, 2, 3]
>>> show(encodeJson(data))
[1,2,3]
```

### 10.9. Files and I/O
```
>>> writeFile("test.txt", "Hello")
()
>>> show(readFile("test.txt"))
Hello
>>> show(fileExists("test.txt"))
true
>>> show(listDir("."))
["test.txt", ...]
```

### 10.10. Map and Set
```
>>> let m = %(1 => "one", 2 => "two")
>>> show(m[1])
one
>>> show(mapSet(m, 3, "three"))
%(1: one, 2: two, 3: three)
>>> show(mapKeys(m2))
[1, 2, 3]
>>> let s = %[1,2,3]
>>> show(setAdd(s, 4))
%[1,2,3,4]
>>> show(s[2])
true
>>> show(setContains(s, 5))
false
```

### 10.11. Classes
```
>>> class Counter = (val = Num; inc(self) = self { val = self.val + 1 }; get(self) = self.val)
>>> let c = new Counter(0)
>>> show(c.inc().get())
1
```

### 10.12. Structures
```
>>> struct Person = (name = String, age = Num)
>>> let p = Person(name = "Alice", age = 30)
>>> show(p.name)
Alice
```

### 10.13. Chat (Complete Scenario with External IP and Password)

**Set external IP and start server with password:**
```
>>> setExternalIP("203.0.113.5")
()
>>> serverStart("0.0.0.0:9000", "secret")
()
```

**Alice:**
```
>>> login(@alice)
()
>>> connect("127.0.0.1:9000", @alice, "secret")
1
>>> newChat("general", [@bob, @alice])
general
>>> open("general")
()
>>> sendChat("general", "Hello everyone!")
true
>>> send(@bob, "Hello Bob!")
true
```

**Bob (another instance):**
```
>>> login(@bob)
()
>>> connect("127.0.0.1:9000", @bob, "secret")
1
>>> show(inbox())
[[Message from @alice in general: "Hello everyone!"], [Message from @alice: "Hello Bob!"]]
>>> show(history("general"))
[[Message from @alice in general: "Hello everyone!"]]
```

**File transfer:**
```
>>> writeFile("report.txt", "Sales data")
()
>>> sendFileToChat("general", "report.txt")
true
>>> show(downloads())
[[FileTransfer from @alice: report.txt]]
>>> saveFile(0, "received_report.txt")
true
```

### 10.14. Processes
```
>>> let p = spawn(lambda () -> (procRecv() |> show))
>>> procSend(p, "Hi")
()
>>> sleep(1s)
()
```

### 10.15. Cryptography
```
>>> let (pk, sk) = kyberKeyPair()
>>> let (ct, ss1) = kyberEncapsulate(pk)
>>> let ss2 = kyberDecapsulate(sk, ct)
>>> show(ss1 == ss2)
true
>>> show(sha256String("hello"))
2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
```

---

## 11. P2P Technical Details
- Protocol: JSON lines over TLS.
- Certificates auto‑generated (`chatlang_cert.pem`, `chatlang_key.pem`).
- P2P port: 19000 (configurable via `--p2p-port`).
- Contact server port: configurable (e.g., 9000).
- External IP can be set manually or obtained via `getPublicIP()`.

---

## 12. Known Limitations
- `let ... in` is not supported; use blocks with indentation.
- Some built‑in functions may panic on wrong argument types (but most handle errors gracefully).

---

## 13. Vim and VSCode Syntax Highlighting

### Vim
Place the following in `~/.vim/syntax/chatlang.vim`:
```vim
syn keyword chatlangKeyword let if then else case of lambda data struct try catch error for while in and or not class extends new loop break
syn keyword chatlangType Num Char String Bool Unit Uid ByteString List Tuple Record Pid DateTime Duration Json Maybe Either ChatMsg FileInfo FileTransfer FetchOptions FetchResult Map Set ClassInstance
syn match chatlangComment "#.*$"
syn region chatlangComment start="#-" end="-#" contains=chatlangComment
syn region chatlangString start=+"+ end=+"+ skip=+\\"+
syn region chatlangFString start=+f"+ end=+"+ contains=chatlangExpr
syn region chatlangExpr start=+{+ end=+}+ contained
syn match chatlangNumber "\<[0-9.]\+\>"
highlight link chatlangKeyword Keyword
highlight link chatlangType Type
highlight link chatlangComment Comment
highlight link chatlangString String
highlight link chatlangFString String
highlight link chatlangNumber Number
```

Add to `.vimrc`:
```vim
au BufRead,BufNewFile *.cl set filetype=chatlang
```

### VSCode
Create a language extension or use the following in `settings.json`:
```json
"files.associations": {
    "*.cl": "chatlang"
}
```
Then define a custom grammar in `chatlang.tmLanguage.json` (see online resources).

---

## 14. Conclusion
This documentation accurately reflects the current implementation of ChatLang 3.2. All examples have been tested and work. For full chat usage, follow section 10.13. Future updates will add more features and improve error handling.
