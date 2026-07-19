## Введение
PLIC — это динамический язык программирования с функциональным ядром и встроенной поддержкой P2P-чата. Он сочетает функциональную, императивную и объектно-ориентированную парадигмы.

Версия 3.2 поддерживает:
- **Каррирование** – все функции каррированы.
- **Тип Num** – объединяет Int и Float.
- **Блоки на основе отступов** – `:` и отступы вместо `{ }`.
- **Генераторы списков** – `[expr for var in iterable if condition]`.
- **Цикл** – `loop { ... }` и `break` с необязательным значением.
- **Чат** – клиент-серверные контакты, локальные чаты, управляющие сообщения для синхронизации.
- **Упоминания** – `@uid` в сообщениях доставляет личную копию.
- **Вывод ошибок** – файл, строка, столбец, фрагмент кода.
- **F-строки** – `f"текст {выражение} текст"` с подсветкой в REPL.
- **`del`** – удаление переменной.
- **`load`** – импорт файлов.
- **Устойчивость к панике** – нет `unwrap`; ошибки обрабатываются.

Все описанные функции реализованы и протестированы.

---

## 1. REPL
Запуск: `cargo run` или `plic`.

- Ввод однострочных выражений; результаты **не выводятся автоматически**. Используйте `show $ expr` для печати значения (она выводит на экран и возвращает `()`).
- Для многострочных блоков используйте двоеточие, затем строки с увеличенным отступом. Завершите блок пустой строкой или словом `end`. Блок возвращает значение последнего выражения.
- Встроенные функции управления REPL:
  - `exit()` – завершает интерпретатор.
  - `load(filename)` – загружает и выполняет скрипт; определения сохраняются.
- Ошибки выводятся с позицией и фрагментом кода.

Пример:
```
>>> let greet name = "Hello, " ++ name ++ "!"
>>> show $ greet "World"
Hello, World!
>>> let square x = x * x:
...     show $ square 5
25
```

---

## 2. Лексика и синтаксис

### Комментарии
- Однострочные: `# текст`
- Многострочные: `#- текст -#` (вложенные поддерживаются)

### Идентификаторы
- Буквы, цифры, `_` (начинаются с буквы или `_`).

### Литералы
- Целые: `42`, `-7`
- Вещественные: `3.14`, `-0.5`, `1.0e10`
- Символы: `'a'`, `'\n'`
- Строки: `"Hello"`, `"line1\nline2"`
- Булевы: `true`, `false`
- Единица: `()`
- UID: `@alice`, `@everyone`
- Списки: `[1, 2, 3]`, `[0..10]` (диапазон, не включая верхнюю границу)
- Кортежи: `(1, "ok")`, `(true, @bob)`
- Байтовые строки: `#B"48656C6C6F"`
- Длительности: `5s`, `500ms`, `2m`, `1h`
- Map-литерал: `%(key => value, ...)`
- Set-литерал: `%[elem, ...]`
- F-строка: `f"текст {выражение} текст"`

---

## 3. Типы значений
- Примитивные: `Num` (Int/Float), `Char`, `String`, `Bool`, `Unit`, `Uid`, `ByteString`, `Pid`, `DateTime`, `Duration`.
- Составные: списки, кортежи, записи.
- Пользовательские: `data`, `struct`, классы.
- Коллекции: `Map` и `Set`.

---

## 4. Выражения

### 4.1. Основы
- Литералы, переменные.
- Применение функции каррировано и левоассоциативно: `f a b c` = `(((f a) b) c)`.
- Лямбда: `lambda x y -> x + y` или `\ x -> x * 2`.
- Индексация: `expr[index]` работает со списками, строками, байтовыми строками, кортежами, map (возвращает значение или `Unit`), set (возвращает `Bool`).

### 4.2. Условный оператор
```
if условие then выражение1 else выражение2
```
Только однострочный.

### 4.3. Сопоставление с образцом
```
case выражение of
    образец1 -> выражение1
    образец2 -> выражение2
    _ -> по умолчанию
```
Однострочный вариант с точками с запятой:
```
case выражение of образец1 -> выражение1; образец2 -> выражение2; _ -> по умолчанию
```

### 4.4. Локальные определения
- `let x = 5` – определяет новую переменную (ошибка, если уже определена).
- `let f x y = x + y` – определение функции (каррировано). Запятые между параметрами **не допускаются**.
- Аннотации типов: `let N :: Num = 5`, `let add :: Num -> Num -> Num = lambda a b -> a + b`.
- Блоки используют отступы после `:`.

### 4.5. Операторы (приоритет, от высокого к низкому)
1. `not` (унарный)
2. `*`, `/`, `%` (левоассоциативные)
3. `+`, `-` (левоассоциативные)
4. `++` (конкатенация строк/списков, правоассоциативная)
5. `:` (cons, правоассоциативная)
6. `in`, `not in` (проверка принадлежности)
7. `==`, `!=`, `<`, `>`, `<=`, `>=`
8. `and`, `or` (короткое замыкание)
9. `|>` (pipe, левоассоциативный)
Оператор `$` поддерживается (низкоприоритетное применение, правоассоциативный).

### 4.6. Циклы
- `for x in коллекция: тело` – итерация по спискам, set, map (для map – пары ключ-значение).
- `while условие: тело`
- `loop { тело }` – бесконечный цикл; используйте `break` с необязательным значением для выхода.

Пример:
```
>>> let x = 0
>>> loop { x = x + 1; if x == 5 then break x else () }
5
```

### 4.7. Обработка ошибок
- `error "сообщение"` – выброс ошибки.
- `try выражение` – перехват ошибки (если есть, передаёт дальше).
- `try выражение catch образец -> обработчик` – обработка ошибки.

Пример:
```
try error "oops" catch e -> "caught: " ++ e
```

### 4.8. F-строки
Синтаксис: `f"текст {выражение} текст"`. Внутри фигурных скобок может быть любое выражение; оно вычисляется и преобразуется в строку через `show`. Двойные скобки `{{` и `}}` используются для вставки литеральных `{` и `}`.

Пример:
```
>>> let x = 5
>>> f"x = {x}, x*2 = {x * 2}"
"x = 5, x*2 = 10"
```

### 4.9. Генераторы списков
```
[expr for var in iterable if condition]
```
Может содержать несколько `for` и `if`.

Пример:
```
>>> [x * 2 for x in [1,2,3] if x > 1]
[4, 6]
```

---

## 5. Структуры и алгебраические типы данных

```
struct Person = (name = String, age = Num)
data Result a = Ok a | Err String
```

Создание структуры: `Person(name = "Alice", age = 30)`. Доступ к полям через `.` (например, `person.name`). Обновление записи: `person { age = 31 }` (возвращает новую запись).

---

## 6. Классы (ООП)

```
class Counter = (val = Num; inc(self) = self { val = self.val + 1 }; get(self) = self.val)
```

- Конструктор: `new Counter(0)`
- Наследование: `class AdvancedCounter extends Counter = (...)` (с круглыми скобками)
- Вызов метода: `counter.inc()`

---

## 7. Union (ADT)

```
data Option a = None | Some a
```

Конструкторы: `Some 42`, `None`. Сопоставление с образцом работает.

---

## 8. Map и Set

### Map
- Создание: `%(key => value, ...)`
- Функции: `mapGet`, `mapSet`, `mapRemove`, `mapKeys`, `mapValues`, `mapEntries`, `mapContains`, `mapSize`, `mapFilter`, `mapMerge`
- Индексация: `map[key]` возвращает значение или `Unit`

### Set
- Создание: `%[elem, ...]`
- Функции: `setAdd`, `setRemove`, `setContains`, `setUnion`, `setIntersection`, `setDifference`, `setSize`, `setFilter`, `setMap`
- Индексация: `set[elem]` возвращает `Bool`

---

## 9. Встроенные функции

### 9.1. Математика
- `sqrt`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan :: Num -> Num`
- `toFloat :: Num -> Num`, `toInt :: Num -> Num`

### 9.2. Преобразования и интроспекция
- `show :: a -> ()` – печатает значение и возвращает `()`.
- `parseInt :: String -> Num`, `parseFloat :: String -> Num`
- `chr :: Num -> Char`, `ord :: Char -> Num`
- `typeof :: a -> String`

### 9.3. Списки, строки, байтовые строки, Map, Set
- `null :: [a] -> Bool`
- `length :: collection -> Num` (работает со списками, строками, байтовыми строками, Map, Set, кортежами)
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
- `sort :: [a] -> [a]` (по строковому представлению)
- `sortBy :: (a -> a -> Num) -> [a] -> [a]`
- `sum :: [Num] -> Num`
- `concat :: [[a]] -> [a]`
- `flatten :: [[a]] -> [a]`
- `zip :: [a] -> [b] -> [(a,b)]`
- `zipWith :: (a -> b -> c) -> [a] -> [b] -> [c]`
- `unzip :: [(a,b)] -> ([a],[b])`
- `indexOf :: [a] -> a -> Maybe Num`
- `lastIndexOf :: [a] -> a -> Maybe Num`

### 9.4. Строковые функции
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

### 9.6. Время
- `formatTime :: String -> DateTime -> String`
- `parseTime :: String -> String -> DateTime`
- `addDuration :: DateTime -> Duration -> DateTime`
- `diffDuration :: DateTime -> DateTime -> Duration`
- `now :: () -> DateTime`

### 9.7. Байтовые строки
- `packBytes :: [Num] -> ByteString`
- `unpackBytes :: ByteString -> [Num]`

### 9.8. Ввод-вывод и файлы
- `putStrLn :: String -> ()` – печатает строку.
- `getLine :: () -> String`
- `getArgs :: () -> [String]`
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

### 9.9. Сеть
- `fetch :: String -> FetchResult`
- `fetchOpts :: FetchOptions -> FetchResult`

### 9.10. Чат и контакты
- `login :: Uid -> ()`
- `logout :: () -> ()`
- `deleteUser :: Uid -> ()`
- `newChat :: String -> [Uid] -> String`
- `addMember :: Uid -> String -> ()`
- `removeMember :: Uid -> String -> ()`
- `deleteChat :: String -> ()`
- `open :: String -> ()`
- `send :: Uid -> String -> Bool` (личное сообщение)
- `sendFile :: Uid -> String -> Bool`
- `sendChat :: String -> String -> Bool` – отправка в чат с поддержкой упоминаний.
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

### 9.11. Процессы
- `spawn :: (() -> ()) -> Pid`
- `procSelf :: () -> Pid`
- `procSend :: Pid -> a -> ()`
- `procRecv :: () -> a`
- `procWait :: Pid -> a`
- `procExit :: a -> ()`
- `sleep :: Duration -> ()`
- `after :: Duration -> (() -> ()) -> ()`

### 9.12. Maybe
- `Nothing :: Maybe a`
- `Just :: a -> Maybe a`
- `maybe :: (a -> b) -> b -> Maybe a -> b`

### 9.13. Map и Set (дополнительно)
- `mapGet`, `mapSet`, `mapRemove`, `mapKeys`, `mapValues`, `mapEntries`, `mapContains`, `mapSize`, `mapFilter`, `mapMerge`
- `setAdd`, `setRemove`, `setContains`, `setUnion`, `setIntersection`, `setDifference`, `setSize`, `setFilter`, `setMap`
- `listToSet :: [a] -> Set`
- `mapToList :: Map -> [(key, value)]`

### 9.14. Криптография
- `sha256 :: ByteString -> ByteString`
- `sha256String :: String -> String`
- `kyberKeyPair :: () -> (PublicKey, SecretKey)` (оба как ByteString)
- `kyberEncapsulate :: PublicKey -> (Ciphertext, SharedSecret)`
- `kyberDecapsulate :: SecretKey -> Ciphertext -> SharedSecret`

### 9.15. Управление переменными
- `del :: String -> ()` – удаляет переменную.
- `load :: String -> ()` – загружает и выполняет файл.

### 9.16. P2P
- `p2pPort :: () -> Num` – возвращает текущий P2P-порт.

---

## 10. Примеры

### 10.1. Арифметика и функции
```
>>> show $ 1 + 2 * 3
7
>>> let sq x = x * x
>>> show $ sq 5
25
>>> show $ (lambda x -> x + 1) 5
6
>>> show $ -5
-5
>>> show $ 5 - -2
7
```

### 10.2. Переменные и присваивание
```
>>> let x = 5
>>> show x
5
>>> let x = 6
error: variable 'x' already defined
>>> x = 10
>>> show x
10
```

### 10.3. Символы и строки
```
>>> show 'a'
a
>>> show '\n'

>>> let msg = "Hello"
>>> show msg
Hello
>>> show $ msg ++ " world"
Hello world
```

### 10.4. Аннотации типов и typeof
```
>>> let N :: Num = 5
>>> show $ typeof N
Num
>>> let add :: Num -> Num -> Num = lambda a b -> a + b
>>> show $ typeof add
Closure
>>> let msg :: String = "hello"
>>> show $ typeof msg
String
>>> N = 'a'   # type mismatch
error: Type mismatch: expected 'Num', got 'Char'
```

### 10.5. Условный оператор
```
>>> show $ if 5 > 3 then "yes" else "no"
yes
```

### 10.6. Списки, строки, кортежи
```
>>> show $ [1, 2, 3] |> map(lambda x -> x * 2)
[2, 4, 6]
>>> show $ "Hello, " ++ "world!"
Hello, world!
>>> show $ length "abc"
3
>>> show $ [0..5]
[0, 1, 2, 3, 4]
>>> show $ (1, "two", 3.0)[1]
two
>>> show $ length (1,2,3)
3
```

### 10.7. Сопоставление с образцом
```
>>> show $ case 2 of 1 -> "one"; 2 -> "two"; _ -> "other"
two
```

### 10.8. JSON
```
>>> let data = parseJson "[1, 2, 3]"
>>> show data
[1, 2, 3]
>>> show $ encodeJson data
[1,2,3]
```

### 10.9. Файлы и ввод-вывод
```
>>> writeFile "test.txt" "Hello"
()
>>> show $ readFile "test.txt"
Hello
>>> show $ fileExists "test.txt"
true
>>> show $ listDir "."
["test.txt", ...]
```

### 10.10. Map и Set
```
>>> let m = %(1 => "one", 2 => "two")
>>> show $ m[1]
one
>>> show $ mapSet m 3 "three"
%(1: one, 2: two, 3: three)
>>> show $ mapKeys m
[1, 2, 3]
>>> let s = %[1,2,3]
>>> show $ setAdd s 4
%[1,2,3,4]
>>> show $ s[2]
true
>>> show $ setContains s 5
false
```

### 10.11. Классы
```
>>> class Counter = (val = Num; inc(self) = self { val = self.val + 1 }; get(self) = self.val)
>>> let c = new Counter(0)
>>> show $ c.inc().get()
1
```

### 10.12. Структуры
```
>>> struct Person = (name = String, age = Num)
>>> let p = Person(name = "Alice", age = 30)
>>> show $ p.name
Alice
```

### 10.13. Чат (полный сценарий с внешним IP и паролем)

**Установка внешнего IP и запуск сервера с паролем:**
```
>>> setExternalIP "203.0.113.5"
>>> serverStart "0.0.0.0:9000" "secret"
```

**Алиса:**
```
>>> login @alice
>>> connect "127.0.0.1:9000" @alice "secret"
>>> newChat "general" [@bob, @alice]
>>> open "general"
>>> sendChat "general" "Hello everyone!"
>>> send @bob "Hello Bob!"
```

**Боб (другой экземпляр):**
```
>>> login @bob
>>> connect "127.0.0.1:9000" @bob "secret"
>>> show inbox
[[Message from @alice in general: "Hello everyone!"], [Message from @alice: "Hello Bob!"]]
>>> show $ history "general"
[[Message from @alice in general: "Hello everyone!"]]
```

**Передача файла:**
```
>>> writeFile "report.txt" "Sales data"
>>> sendFileToChat "general" "report.txt"
>>> show downloads
[[FileTransfer from @alice: report.txt]]
>>> saveFile 0 "received_report.txt"
```

### 10.14. Процессы
```
>>> let p = spawn lambda () -> (procRecv |> show)
>>> procSend p "Hi"
>>> sleep 1s
```

### 10.15. Криптография
```
>>> let (pk, sk) = kyberKeyPair()
>>> let (ct, ss1) = kyberEncapsulate pk
>>> let ss2 = kyberDecapsulate sk ct
>>> show $ ss1 == ss2
true
>>> show $ sha256String "hello"
2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
```

---

## 11. Технические детали P2P
- Протокол: JSON-строки поверх TLS.
- Сертификаты генерируются автоматически (`plic_cert.pem`, `plic_key.pem`).
- P2P-порт: 19000 (можно изменить с помощью `--p2p-port`).
- Порт сервера контактов: настраивается (например, 9000).
- Внешний IP можно задать вручную или получить через `getPublicIP()`.

---

## 12. Известные ограничения
- `let ... in` не поддерживается; используйте блоки с отступами.
- Некоторые встроенные функции могут паниковать при неверных типах аргументов (большинство обрабатывают ошибки).

---

## 13. Подсветка синтаксиса для Vim и VSCode

### Vim
Поместите содержимое `hl/vim/syntax/plic.vim` в `~/.vim/syntax/plic.vim`, а `hl/vim/ftdetect/plic.vim` – в `~/.vim/ftdetect/plic.vim`. Добавьте в `.vimrc`:
```vim
au BufRead,BufNewFile *.plic set filetype=plic
```

### VSCode
Скопируйте папку `hl/vsc` в `.vscode` вашего проекта или используйте расширение. В `settings.json` добавьте:
```json
"files.associations": {
    "*.plic": "plic"
}
```
