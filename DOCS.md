# Evelyn Language Reference (Avelyn v2.5.7)

> **Evelyn** (runtime & compiler: **Avelyn**) is a modern, dynamically-typed, indentation-aware programming language with an expressive Python/Swift-inspired syntax, a rich standard library, and dual execution modes: an instant **high-performance interpreter** and an **AOT native compiler pipeline** generating standalone, highly-optimized machine binaries.

---

## Table of Contents

1. [Getting Started](#1-getting-started)
2. [Core Syntax](#2-core-syntax)
3. [Control Flow](#3-control-flow)
4. [Functions](#4-functions)
5. [Error Handling](#5-error-handling)
6. [Modules & Imports](#6-modules--imports)
7. [Built-in Functions](#7-built-in-functions)
8. [Standard Library](#8-standard-library)
   - [string](#81-string)
   - [array](#82-array)
   - [math](#83-math)
   - [io & path](#84-io--path)
   - [json](#85-json)
   - [http](#86-http)
   - [web](#87-web)
   - [middleware](#88-middleware)
   - [async](#89-async)
   - [datetime](#810-datetime)
   - [random](#811-random)
   - [re (Regex)](#812-re-regex)
   - [collections](#813-collections)
   - [functional](#814-functional)
   - [logger](#815-logger)
   - [validate](#816-validate)
   - [test](#817-test)
   - [cli](#818-cli)
   - [term](#819-term)
   - [prompt](#820-prompt)
   - [template](#821-template)
   - [config](#822-config)
   - [uuid](#823-uuid)
   - [secrets](#824-secrets)
   - [lzw](#825-lzw)
   - [pathlib](#826-pathlib)
   - [sys & os](#827-sys--os)
   - [hashlib, base64 & hex](#828-hashlib-base64--hex)
   - [dotenv](#829-dotenv)
   - [glob & shutil](#830-glob--shutil)
   - [platform & time](#831-platform--time)
9. [Complete Example Programs](#9-complete-example-programs)

---

## 1. Getting Started

### Running a Script

```
avelyn hello.lyn
```

### Hello World

```lyn
print("Hello, World!")
```

### Importing the Full Standard Library

```lyn
import "init"   # loads every stdlib module at once
```

Or import individual modules:

```lyn
import "math"
import "string"
import "json"
```

---

## 2. Syntax Mechanics & Runtime Behavior

Evelyn is engineered around an intuitive, expressive grammar that blends the clean readability of Python with the functional power and safety of Swift.

---

### 2.1 Lexical Grammar & Block Mechanics

#### Dual Block Styles: Indentation vs Braces
Evelyn supports both **indentation-aware blocks** (colon `:` followed by indented lines) and **explicit brace blocks** (`{ ... }`):

```lyn
# Indentation-based block (Python style)
if count > 10:
    let factor = 2
    total += count * factor

# Brace-delimited block (C/Swift style)
if count > 10 {
    let factor = 2
    total += count * factor
}
```

#### Statement Termination & Multi-line Constructs
- **Newlines** naturally terminate statements.
- **Semicolons (`;`)** can optionally be used to place multiple statements on a single line:
  ```lyn
  let a = 1; let b = 2; let c = a + b
  ```
- Multi-line arrays, maps, and parenthesized expressions automatically continue across lines without requiring trailing backslashes:
  ```lyn
  let matrix = [
      [1, 2, 3],
      [4, 5, 6],
      [7, 8, 9]
  ]
  ```

#### Literals & Number Formats
Evelyn supports first-class numeric literals across various radices, with underscore `_` visual separators:

```lyn
let decimal_val  = 1_000_000     # Decimal with separators
let hex_val      = 0xDEAD_BEEF   # Hexadecimal (base 16)
let binary_val   = 0b1011_0010   # Binary (base 2)
let octal_val    = 0o755         # Octal (base 8)
let float_val    = 3.14159265    # IEEE 754 64-bit float
let sci_val      = 2.5e-4        # Scientific notation (0.00025)
```

#### String Literals, Escapes & Interpolation
Strings in Evelyn are UTF-8 encoded and support standard escape sequences:
- `\n` (Newline), `\t` (Tab), `\r` (Carriage return), `\0` (Null byte), `\\` (Backslash), `\"` (Double quote), `\'` (Single quote).
- **String Interpolation**: Expressions embedded inside `\(...)` are evaluated at runtime and converted to strings:
  ```lyn
  let user = "Alice"
  let score = 95
  print("User: \(user), Score: \(score + 5)/100")
  ```

---

### 2.2 Variable Bindings, Scoping & Mutability

#### `let` (Immutable) vs `var` (Mutable)
- **`let`**: Declares an immutable binding. Once assigned, reassigning to a `let` variable triggers a compilation error / runtime exception:
  ```lyn
  let maxRetries = 5
  # maxRetries = 6  <-- ERROR: Cannot reassign immutable binding
  ```
- **`var`**: Declares a mutable variable that can be reassigned and mutated with compound operators:
  ```lyn
  var balance = 100
  balance += 50    # balance = 150
  balance -= 20    # balance = 130
  balance *= 2     # balance = 260
  balance /= 4     # balance = 65
  balance %= 10    # balance = 5
  ```

#### Compound Bitwise Operators
`var` integers can be modified directly with bitwise compound operators:
`&=` (AND), `|=` (OR), `^=` (XOR), `<<=` (Left Shift), `>>=` (Right Shift).

#### Lexical Block Scoping & Variable Shadowing
Variables are strictly scoped to the block in which they are defined. Inner scopes can shadow outer variables without overwriting them:

```lyn
let x = 10
if true:
    let x = 99      # Shadows outer x for this block
    print(x)        # 99

print(x)            # 10 (outer x preserved)
```

---

### 2.3 Destructuring Assignment

Evelyn provides pattern destructuring for arrays and maps:

#### Array Destructuring
```lyn
let [first, second, third] = [10, 20, 30]
print(first)   # 10
print(second)  # 20

# Swapping variables via array destructuring
var a = 1
var b = 2
let [tmpA, tmpB] = [b, a]
a = tmpA
b = tmpB
```

#### Map Destructuring
Map keys are mapped directly into local variable bindings:
```lyn
let userMap = {"id": 101, "username": "evelyn_dev", "role": "admin"}
let {"id": uid, "username": uName} = userMap

print(uid)     # 101
print(uName)   # evelyn_dev
```

#### Destructuring in Loops
```lyn
let points = [[0, 0], [10, 20], [30, 40]]
for point in points:
    let [x, y] = point
    print("Point: (\(x), \(y))")
```

---

### 2.4 Truthiness & Logic Semantics

#### Truthiness Rules
In Evelyn, the following values are evaluated as **falsy** in conditionals:
- `false` (Boolean false)
- `null` (Null value)
- `0` and `0.0` (Zero numbers)
- `""` (Empty string)
- `[]` (Empty array)
- `{}` (Empty map)

All other values (including non-zero numbers, non-empty strings, collections, structs, and functions) are **truthy**.

#### Logical Operators & Short-Circuit Evaluation
- **`and` / `&&`**: Evaluates the right operand only if the left operand is truthy.
- **`or` / `||`**: Evaluates the right operand only if the left operand is falsy.
- **`not` / `!`**: Inverts boolean truthiness.

```lyn
# Short-circuit guarantee: fnTrigger() is never executed
let res = false and fnTrigger()
```

#### Null-Coalescing Operator (`??`)
Provides a clean fallback value when dealing with `null`:
```lyn
let configuredPort = null
let port = configuredPort ?? 8080   # port = 8080
```

#### Ternary Conditional Operator (`? :`)
Inline conditional expressions:
```lyn
let status = age >= 18 ? "Adult" : "Minor"
```

---

### 2.5 Functions, Closures & Execution Pipeline

#### Named Function Declarations & Default Arguments
Functions can be declared using either `fn` or `def`:

```lyn
fn calculateTotal(subtotal, taxRate = 0.05, discount = 0.0):
    return (subtotal - discount) * (1.0 + taxRate)

let total = calculateTotal(100.0)             # uses defaults: (100 - 0) * 1.05 = 105.0
let custom = calculateTotal(100.0, 0.10, 10.0) # (100 - 10) * 1.10 = 99.0
```

#### Variadic Parameters (`...args`)
Collects variable numbers of arguments into a dynamic array:

```lyn
fn sumValues(...values):
    var sum = 0
    for v in values: sum += v
    return sum

print(sumValues(1, 2, 3, 4, 5))   # 15
```

#### Anonymous Functions (Lambdas)
- **Arrow Expression Form**: `def(x) => x * 2`
- **Block Form**: `def(x) { let r = x * 2; return r }`

#### Lexical Closures & State Accumulators
Functions capture their enclosing environment by reference, allowing stateful generators:

```lyn
fn createCounter(initial = 0):
    var count = initial
    return def(step = 1) {
        count += step
        return count
    }

let counterA = createCounter(10)
print(counterA(5))    # 15
print(counterA(5))    # 20
```

#### Forward Pipe Operator (`|>`)
Pipelines transform values linearly without deeply nested function calls:

```lyn
fn addFive(x): return x + 5
fn double(x): return x * 2
fn square(x): return x * x

# Evaluates as: square(double(addFive(5)))
let result = 5 |> addFive |> double |> square
print(result)   # ( (5 + 5) * 2 )^2 = 20^2 = 400
```

---

### 2.6 Pattern Matching & Enums

#### `match` and `switch` Expressions
Pattern matching checks expressions against values, ranges, and types without fallthrough:

```lyn
fn evaluateCode(status):
    match status {
        case 200:
            return "SUCCESS"
        case 400...499:
            return "CLIENT_ERROR"
        case 500...599:
            return "SERVER_ERROR"
        default:
            return "UNKNOWN_STATUS"
    }
```

#### Enums & Algebraic Data Types (ADTs)
Enums support both unit variants and variants carrying payload values:

```lyn
enum OrderStatus {
    Created,
    Processing(workerId),
    Shipped(trackingNumber),
    Cancelled(reason)
}

let myOrder = OrderStatus.Shipped("TRACK_998124")

match myOrder {
    case OrderStatus.Created:
        print("Order has been created")
    case OrderStatus.Processing(worker):
        print("Processing by worker: \(worker)")
    case OrderStatus.Shipped(tracking):
        print("Shipped with tracking: \(tracking)")
    case OrderStatus.Cancelled(reason):
        print("Cancelled: \(reason)")
}
```

---

### 2.7 Structs & Object Representation

Structs provide lightweight record types with named fields:

```lyn
struct Point3D {
    x,
    y,
    z
}

let pt = Point3D(10, 20, 30)
print("X: \(pt.x), Y: \(pt.y), Z: \(pt.z)")

# Mutating struct instances
pt.x = 99
print(pt.x)   # 99
```

---

### 2.8 Error Handling & Exception Lifecycle

Evelyn uses structured `try` / `catch` / `finally` blocks for reliable error recovery and deterministic resource cleanup:

```lyn
var fileHandle = "OPEN_FILE_DESCRIPTOR"

try {
    # Thrown values can be strings, numbers, maps, or structs
    if errorOccurred:
        throw {"code": 503, "message": "ServiceUnavailable"}
} catch err {
    print("Caught error: \(toString(err))")
} finally {
    # Finally blocks are guaranteed to execute even if errors are rethrown
    fileHandle = "CLOSED"
    print("Resource closed safely.")
}
```

---

## 3. Control Flow

### 3.1 if / else if / else

Both **indentation-delimited** (Python-style) and **brace-delimited** (C-style) blocks work:

```lyn
# Indentation style
if x > 0:
    print("positive")
else if x < 0:
    print("negative")
else:
    print("zero")

# Brace style
if x > 0 {
    print("positive")
} else {
    print("zero")
}
```

---

### 3.2 while

```lyn
var i = 0
while i < 5:
    print(i)
    i += 1
# prints 0 1 2 3 4
```

---

### 3.3 for / in (collections)

```lyn
let fruits = ["apple", "banana", "cherry"]
for fruit in fruits:
    print(fruit)

# Iterating a string gives individual characters
for ch in "hello":
    print(ch)
```

---

### 3.4 for / in (ranges)

```lyn
for i in 0..5:      # exclusive: 0,1,2,3,4
    print(i)

for i in 1...5:     # inclusive: 1,2,3,4,5
    print(i)
```

---

### 3.5 switch / match

Both `switch` and `match` are identical keywords. Cases do **not** fall through.

```lyn
let day = "Monday"
switch day {
    case "Monday":
        print("Start of the week")
    case "Friday":
        print("End of the week")
    default:
        print("Midweek")
}

let code = 404
match code {
    case 200: print("OK")
    case 404: print("Not Found")
    case 500: print("Server Error")
    default:  print("Unknown")
}
```

---

### 3.6 break & continue

```lyn
for i in 1...10:
    if i == 5: break       # exit loop
    print(i)               # 1 2 3 4

for i in 1...10:
    if i % 2 == 0: continue  # skip even
    print(i)               # 1 3 5 7 9
```

---

## 4. Functions

### 4.1 Named Functions

```lyn
def greet(name) {
    return "Hello, " + name + "!"
}
print(greet("Alice"))   # Hello, Alice!

# Indentation style
def add(a, b):
    return a + b

print(add(3, 4))   # 7
```

A function with no explicit `return` returns `null`.

---

### 4.2 Anonymous Functions (Lambdas)

**Arrow form** — single-expression body:
```lyn
let double = def(x) => x * 2
print(double(5))   # 10
```

**Block form** — multi-statement body:
```lyn
let square = def(n) {
    let result = n * n
    return result
}
print(square(7))   # 49
```

---

### 4.3 First-Class Functions

Functions are values — store, pass, and return them freely:

```lyn
def apply(fn, value) {
    return fn(value)
}
let triple = def(x) => x * 3
print(apply(triple, 10))   # 30

# In a map
let ops = {
    "add": def(a, b) => a + b,
    "mul": def(a, b) => a * b,
}
print(ops["add"](10, 5))   # 15
print(ops["mul"](3, 7))    # 21
```

---

### 4.4 Variadic Functions

Use `...name` to collect extra arguments into an array:

```lyn
def sum(...nums) {
    var total = 0
    for n in nums: total += n
    return total
}
print(sum(1, 2, 3, 4, 5))   # 15

# Fixed params before variadic
def log(level, ...messages) {
    for msg in messages:
        print("[" + level + "] " + msg)
}
log("INFO", "Server started", "Port 8080")
```

---

### 4.5 Closures

Functions capture their surrounding lexical scope:

```lyn
def makeCounter() {
    var count = 0
    def increment() {
        count += 1
        return count
    }
    return increment
}

let counter = makeCounter()
print(counter())   # 1
print(counter())   # 2
print(counter())   # 3
```

---

## 5. Error Handling & Diagnostics

### 5.1 Structured Exception Handling: `try` / `catch` / `finally`

```lyn
try {
    throw "Something went wrong"
} catch e {
    print("Caught: " + e)
} finally {
    print("This always runs")
}
```

**Custom error objects:**

```lyn
def safeDivide(a, b) {
    if b == 0 {
        throw {"code": 400, "message": "Division by zero"}
    }
    return a / b
}

try {
    print(safeDivide(10, 0))
} catch e {
    print("Error " + numToString(e["code"]) + ": " + e["message"])
}
```

> The caught variable `e` holds whatever value was thrown — string, number, map, or any value.

**Re-throwing:**

```lyn
try {
    riskyOperation()
} catch e {
    logError(e)
    throw e   # propagate upward
}
```

---

### 5.2 Python-Style Error Traceback & Diagnostics

When an unhandled runtime error or exception occurs, Evelyn prints a comprehensive, Python-style traceback detailing the file, line number, call stack frames, offending source code line, and categorized error type:

```text
Traceback (most recent call last):
  File "calculator.lyn", line 12, in processItem
    let subtotal = price * count
  File "main.lyn", line 45, in <main>
    processItem("Widget", undefined_quantity)
NameError: variable 'undefined_quantity' is not defined
```

#### Standard Error Categories

| Error Category | Trigger Condition | Example Output |
|---|---|---|
| **`NameError`** | Accessing an undeclared or out-of-scope variable | `NameError: variable 'foo' is not defined` |
| **`TypeError`** | Applying incompatible types to operators or functions | `TypeError: cannot apply operator '+' to int and string` |
| **`ZeroDivisionError`** | Division or modulo by zero (`/`, `//`, `%`) | `ZeroDivisionError: division by zero` |
| **`IndexError`** | Accessing array indices out of bounds | `IndexError: array index out of bounds (index 5, length 3)` |
| **`KeyError`** | Accessing non-existent required map keys | `KeyError: key 'username' not found` |
| **`AssertionError`** | Failed `assert` conditions | `AssertionError: expected score 100, got 50` |
| **`MutabilityError`** | Attempting to reassign an immutable `let` binding | `MutabilityError: Cannot assign to immutable binding 'maxCount'` |
| **`UncaughtException`** | Unhandled thrown values | `UncaughtException: DatabaseConnectionTimeout` |
| **`SyntaxError`** | Parser unexpected tokens or missing delimiters | `SyntaxError: expected ':' or '{' after if condition` |
| **`RuntimeError`** | General runtime failure | `RuntimeError: stack overflow in recursive call` |

---

## 6. Modules & Imports

```lyn
import "math"           # stdlib module (no .lyn needed)
import "string"
import "json"

import "utils.lyn"      # local file
import "./helpers/fmt"  # relative path

import "init"           # load ALL stdlib modules at once
```

After importing, module variables are available:

```lyn
import "math"
print(math["pi"])           # 3.141592653589793
print(math["floor"](3.7))   # 3
```

---

## 7. Built-in Functions

These are always available, no import needed.

### Type Checking

| Function | Returns | Description |
|----------|---------|-------------|
| `isNull(v)` | bool | `true` if `v` is `null` |
| `isString(v)` | bool | `true` if string |
| `isNumber(v)` | bool | `true` if number |
| `isInteger(v)` | bool | `true` if whole number |
| `isBool(v)` | bool | `true` if boolean |
| `isArray(v)` | bool | `true` if array |
| `isMap(v)` | bool | `true` if map |
| `typeOf(v)` | string | Returns type name string |

```lyn
print(isNull(null))       # true
print(isString("hi"))     # true
print(isNumber(3.14))     # true
print(isArray([1,2,3]))   # true
print(isMap({"a":1}))     # true
print(typeOf("hello"))    # string
```

### Type Conversion

| Function | Description |
|----------|-------------|
| `toString(v)` | Any value → string |
| `numToString(v)` | Number → string (no trailing `.0` for integers) |
| `stringToNum(s)` | Parse string → number (`null` if invalid) |
| `toNumber(v)` | Alias for numeric conversion |
| `toBool(v)` | Truthy coercion |

```lyn
print(toString(42))        # 42
print(toString([1,2,3]))   # [1, 2, 3]
print(numToString(10.0))   # 10
print(stringToNum("3.14")) # 3.14
```

### String Built-ins

| Function | Description |
|----------|-------------|
| `stringLen(s)` | Length |
| `stringSub(s, start, len)` | Substring at start of given length |
| `stringSplit(s, sep)` | Split by separator |
| `stringJoin(arr, sep)` | Join array with separator |
| `stringTrim(s)` | Strip whitespace |
| `stringToUpper(s)` | Uppercase |
| `stringToLower(s)` | Lowercase |
| `stringContains(s, sub)` | Substring check |
| `stringIndexOf(s, sub)` | Index of sub, -1 if absent |
| `stringReplaceAll(s, old, new)` | Replace all occurrences |
| `stringAt(s, idx)` | Character at index |
| `stringRepeat(s, n)` | Repeat n times |
| `stringPadStart(s, w, ch)` | Pad start to width |
| `stringPadEnd(s, w, ch)` | Pad end to width |

```lyn
print(stringLen("hello"))                  # 5
print(stringSub("hello world", 6, 5))     # world
print(stringSplit("a,b,c", ","))           # [a, b, c]
print(stringJoin(["a","b","c"], "-"))      # a-b-c
print(stringTrim("  hi  "))               # hi
print(stringToUpper("hello"))             # HELLO
print(stringReplaceAll("aaa","a","b"))    # bbb
```

### Array Built-ins

| Function | Description |
|----------|-------------|
| `arrayLen(arr)` | Length |
| `arrayAppend(arr, item)` | Add to end, returns new array |
| `arrayPop(arr)` | Remove last |
| `arrayShift(arr)` | Remove first |
| `arraySlice(arr, start, end)` | Subarray (exclusive end) |
| `arrayContains(arr, item)` | Membership test |
| `arrayReverse(arr)` | Reversed copy |
| `arraySort(arr)` | Sorted copy |
| `arrayCopy(arr)` | Shallow copy |
| `arrayMap(arr, fn)` | Map with function |
| `arrayFilter(arr, fn)` | Filter with predicate |
| `arrayReduce(arr, fn, init)` | Reduce to single value |
| `arrayFind(arr, fn)` | First match |
| `arrayFindIndex(arr, fn)` | Index of first match |

```lyn
var nums = [3,1,4,1,5]
arrayAppend(nums, 9)
print(arrayLen(nums))          # 6
print(arraySort(nums))         # [1,1,3,4,5,9]
print(arrayReverse(nums))      # [9,5,1,4,1,3]

let doubled = arrayMap([1,2,3], def(x) => x * 2)
print(doubled)   # [2, 4, 6]

let evens = arrayFilter([1,2,3,4], def(x) => x % 2 == 0)
print(evens)   # [2, 4]

let total = arrayReduce([1,2,3,4], def(acc, x) => acc + x, 0)
print(total)   # 10
```

### Map Built-ins

| Function | Description |
|----------|-------------|
| `mapKeys(m)` | Array of keys |
| `mapGet(m, key)` | Get value |
| `mapSet(m, key, val)` | Set value |
| `mapHas(m, key)` | Key exists? |
| `mapDelete(m, key)` | Delete key |
| `mapCopy(m)` | Shallow copy |
| `mapLen(m)` | Number of keys |
| `mapMerge(m1, m2)` | Merge, m2 wins on conflict |

```lyn
let cfg = {"host": "localhost", "port": 8080}
print(mapKeys(cfg))           # [host, port]
mapSet(cfg, "debug", true)
print(mapHas(cfg, "debug"))   # true
mapDelete(cfg, "debug")
print(mapLen(cfg))             # 2
```

### Math Built-ins

| Function | Description |
|----------|-------------|
| `mathFloor(x)` | Round down |
| `mathCeil(x)` | Round up |
| `mathRound(x)` | Round to nearest |
| `mathSqrt(x)` | Square root |
| `mathLog(x)` / `mathLog2(x)` / `mathLog10(x)` | Logarithms |
| `mathSin(x)` / `mathCos(x)` / `mathTan(x)` | Trig (radians) |

### I/O Built-ins

| Function | Description |
|----------|-------------|
| `print(v)` | Print to stdout |
| `fileRead(path)` | Read file as string |
| `fileWrite(path, data)` | Write string to file |
| `fileAppend(path, data)` | Append to file |
| `fileExists(path)` | File existence check |
| `dirCreate(path)` | Create directory |
| `dirExists(path)` | Directory existence check |
| `dirList(path)` | List directory contents |

### JSON Built-ins

| Function | Description |
|----------|-------------|
| `jsonParse(s)` | Parse JSON string |
| `jsonStringify(v)` | Serialize to JSON string |

### Time Built-ins

| Function | Description |
|----------|-------------|
| `now()` | High-res timestamp (nanoseconds) |
| `timeMs()` | Current time in milliseconds |
| `timeSleep(ms)` | Sleep for ms milliseconds |
| `dateNow()` | Current Unix timestamp (ms) |
| `dateFormat(ts, fmt)` | Format timestamp |
| `dateParse(s, fmt)` | Parse date string |
| `dateAdd(ts, amount, unit)` | Add time |

### Misc Built-ins

| Function | Description |
|----------|-------------|
| `deepEqual(a, b)` | Deep structural equality |
| `deepCopy(v)` | Deep copy |
| `charFromCode(n)` | ASCII code → character |
| `charCode(c)` | Character → ASCII code |
| `uuidV4()` | Generate UUID v4 |
| `urlEncode(s)` | URL-encode |
| `urlDecode(s)` | URL-decode |
| `sysEnv(key)` | Read env variable |
| `sysArgv()` | Command-line args |
| `sysExit(code)` | Exit process |
| `sysReadLine()` | Read line from stdin |
| `sysExecute(cmd, args)` | Run system command |
| `sysRegexMatch(pat, s)` | Regex test |
| `sysRegexReplace(pat, s, repl)` | Regex replace |
| `sysRegexFindAll(pat, s)` | Find all matches |
| `sysRegexGroups(pat, s)` | Capture groups |
| `sysSecureRandomDouble()` | CSPRNG float [0, 1) |
| `sysSecureRandomBytes(n)` | CSPRNG byte string |

---

## 8. Standard Library

---

### 8.1 `string`

```lyn
import "string"
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `repeat` | `(s, n)` | Repeat string n times |
| `startsWith` | `(s, prefix)` | True if s starts with prefix |
| `endsWith` | `(s, suffix)` | True if s ends with suffix |
| `padStart` | `(s, width, ch)` | Pad start to width (default space) |
| `padEnd` | `(s, width, ch)` | Pad end to width |
| `center` | `(s, width, ch)` | Center-align to width |
| `count` | `(s, sub)` | Count non-overlapping occurrences |
| `indexOf` | `(s, sub)` | First index, -1 if absent |
| `replace` | `(s, old, new)` | Replace first occurrence |
| `reverse` | `(s)` | Reverse string |
| `isDigit` | `(s)` | All chars are digits? |
| `isAlpha` | `(s)` | All chars are letters? |
| `isAlphanumeric` | `(s)` | All chars are alphanumeric? |
| `isWhitespace` | `(s)` | All chars are whitespace? |
| `upper` | `(s)` | Uppercase |
| `lower` | `(s)` | Lowercase |
| `trim` | `(s)` | Strip whitespace |
| `split` | `(s, sep)` | Split by separator |
| `join` | `(arr, sep)` | Join array |
| `len` | `(s)` | String length |
| `sub` | `(s, a, b)` | Substring from index a to b (exclusive) |
| `capitalize` | `(s)` | Capitalise first character |
| `camelCase` | `(s)` | snake_case / kebab-case → camelCase |
| `snakeCase` | `(s)` | camelCase / PascalCase → snake_case |

**Examples:**

```lyn
import "string"

print(string["repeat"]("ab", 3))           # ababab
print(string["startsWith"]("hello", "he")) # true
print(string["endsWith"]("hello", "lo"))   # true
print(string["padStart"]("5", 4, "0"))     # 0005
print(string["padEnd"]("hi", 5, "."))      # hi...
print(string["center"]("hi", 7, "-"))      # --hi---
print(string["count"]("banana", "an"))     # 2
print(string["indexOf"]("hello", "ll"))    # 2
print(string["replace"]("aaa", "a", "b")) # baa
print(string["reverse"]("hello"))          # olleh
print(string["isDigit"]("12345"))          # true
print(string["isAlpha"]("abc"))            # true
print(string["isAlphanumeric"]("abc123"))  # true
print(string["upper"]("hello"))            # HELLO
print(string["lower"]("WORLD"))            # world
print(string["trim"]("  hi  "))            # hi
print(string["split"]("a,b,c", ","))       # [a, b, c]
print(string["join"](["x","y","z"], "+"))  # x+y+z
print(string["sub"]("hello", 1, 3))        # el
print(string["capitalize"]("hello world")) # Hello world
print(string["camelCase"]("hello_world"))  # helloWorld
print(string["snakeCase"]("helloWorld"))   # hello_world
```

---

### 8.2 `array`

```lyn
import "array"
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `range` | `(start, stop, step)` | Array from start to stop (exclusive) |
| `sum` | `(arr)` | Sum of elements |
| `min` | `(arr)` | Minimum element |
| `max` | `(arr)` | Maximum element |
| `avg` | `(arr)` | Average |
| `unique` | `(arr)` | Remove duplicates |
| `flatten` | `(arr)` | Recursively flatten nested arrays |
| `zip` | `(a, b)` | Zip two arrays into pairs |
| `chunk` | `(arr, size)` | Split into chunks |
| `groupBy` | `(arr, fn)` | Group by key function |
| `countBy` | `(arr, fn)` | Count by key function |
| `contains` | `(arr, x)` | Membership test |
| `len` | `(arr)` | Length |
| `join` | `(arr, sep)` | Join as string |
| `reverse` | `(arr)` | Reversed copy |
| `sort` | `(arr)` | Sorted copy |
| `slice` | `(arr, a, b)` | Subarray a..b (exclusive) |
| `intersection` | `(a, b)` | Elements in both |
| `difference` | `(a, b)` | Elements in a but not b |
| `union` | `(a, b)` | Combined unique elements |
| `shuffle` | `(arr)` | Randomly shuffled copy |

**Examples:**

```lyn
import "array"

print(array["range"](0, 5, null))          # [0, 1, 2, 3, 4]
print(array["range"](0, 10, 2))            # [0, 2, 4, 6, 8]
print(array["sum"]([1,2,3,4,5]))           # 15
print(array["min"]([3,1,4,1,5]))           # 1
print(array["max"]([3,1,4,1,5]))           # 5
print(array["avg"]([1,2,3,4,5]))           # 3
print(array["unique"]([1,2,2,3,1]))        # [1, 2, 3]
print(array["flatten"]([1,[2,[3,4]],5]))   # [1, 2, 3, 4, 5]
print(array["zip"]([1,2,3],["a","b","c"])) # [[1,a],[2,b],[3,c]]
print(array["chunk"]([1,2,3,4,5], 2))      # [[1,2],[3,4],[5]]

let words = ["cat","dog","cow","ant"]
let grouped = array["groupBy"](words, def(w) => stringSub(w,0,1))
print(grouped)  # {"c":[cat,cow],"d":[dog],"a":[ant]}

print(array["intersection"]([1,2,3],[2,3,4]))  # [2, 3]
print(array["difference"]([1,2,3],[2,3,4]))    # [1]
print(array["union"]([1,2,3],[3,4,5]))         # [1, 2, 3, 4, 5]
```

---

### 8.3 `math`

```lyn
import "math"
```

| Field / Method | Description |
|----------------|-------------|
| `pi` | π ≈ 3.141592653589793 |
| `e` | Euler's number ≈ 2.718281828459045 |
| `inf` | Positive infinity |
| `abs(x)` | Absolute value |
| `min(a, b)` | Minimum of two |
| `max(a, b)` | Maximum of two |
| `clamp(x, lo, hi)` | Clamp x in [lo, hi] |
| `sign(x)` | Returns -1, 0, or 1 |
| `gcd(a, b)` | Greatest common divisor |
| `lcm(a, b)` | Least common multiple |
| `isPrime(n)` | True if n is prime |
| `factorial(n)` | n! |
| `pow(base, exp)` | Integer exponentiation |
| `round(x)` | Round to nearest integer |
| `ceil(x)` | Ceiling |
| `floor(x)` | Floor |

**Examples:**

```lyn
import "math"

print(math["pi"])              # 3.141592653589793
print(math["abs"](-42))        # 42
print(math["clamp"](15, 0, 10)) # 10
print(math["gcd"](48, 18))     # 6
print(math["lcm"](4, 6))       # 12
print(math["isPrime"](17))     # true
print(math["factorial"](5))    # 120
print(math["pow"](2, 10))      # 1024
print(math["floor"](3.9))      # 3
print(math["ceil"](3.1))       # 4
print(math["round"](3.5))      # 4

# Also via built-in operators:
print(2 ** 8)    # 256
print(7 // 2)    # 3
```

---

### 8.4 `io` & `path`

```lyn
import "io"
```

#### `io` module

| Method | Signature | Description |
|--------|-----------|-------------|
| `read` | `(path)` | Read file as string |
| `write` | `(path, data)` | Write string (overwrites) |
| `append` | `(path, data)` | Append to file |
| `readLines` | `(path)` | Read file as array of lines |
| `writeLines` | `(path, lines)` | Write lines joined by `\n` |
| `appendLine` | `(path, line)` | Append one line |
| `exists` | `(path)` | File exists? |
| `size` | `(path)` | File size in bytes |
| `mkdir` | `(path)` | Create directory |
| `dirExists` | `(path)` | Directory exists? |
| `listDir` | `(path)` | List directory |

#### `path` module

| Method | Signature | Description |
|--------|-----------|-------------|
| `join` | `(parts)` | Join path segments |
| `basename` | `(p)` | Filename |
| `dirname` | `(p)` | Parent directory |
| `ext` | `(p)` | Extension (includes `.`) |
| `absolute` | `(p)` | Absolute path |
| `noExt` | `(p)` | Path without extension |

**Examples:**

```lyn
import "io"

io["write"]("hello.txt", "Hello, World!")
let content = io["read"]("hello.txt")
print(content)   # Hello, World!

io["writeLines"]("data.txt", ["line1", "line2", "line3"])
let lines = io["readLines"]("data.txt")
print(lines[0])  # line1

io["mkdir"]("mydir")
print(io["dirExists"]("mydir"))  # true

print(path["basename"]("/usr/local/file.txt"))  # file.txt
print(path["dirname"]("/usr/local/file.txt"))   # /usr/local
print(path["ext"]("/usr/local/file.txt"))        # .txt
print(path["join"](["/usr","local","file.txt"])) # /usr/local/file.txt
```

---

### 8.5 `json`

```lyn
import "json"
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `loads` | `(s)` | Parse JSON string |
| `dumps` | `(v)` | Serialize to JSON string |
| `pretty` | `(v)` | Pretty-print with 2-space indent |
| `merge` | `(m1, m2)` | Deep merge (m2 wins) |
| `get` | `(obj, path)` | Get value by dot-path `"a.b.c"` |
| `set` | `(obj, path, value)` | Set value by dot-path |
| `flatten` | `(obj)` | Flatten nested map to dot-keys |
| `validate` | `(s)` | True if valid JSON |

**Examples:**

```lyn
import "json"

let data = json["loads"]("{\"name\":\"Alice\",\"age\":30}")
print(data["name"])          # Alice
print(json["dumps"](data))   # {"name":"Alice","age":30}

let obj = {"users": [{"id": 1, "name": "Bob"}]}
print(json["pretty"](obj))
# {
#   "users": [
#     {
#       "id": 1,
#       "name": "Bob"
#     }
#   ]
# }

let config = {"db": {"host": "localhost", "port": 5432}}
print(json["get"](config, "db.host"))    # localhost
json["set"](config, "db.name", "mydb")
print(json["get"](config, "db.name"))    # mydb

let base = {"a": 1, "b": {"x": 10}}
let over = {"b": {"y": 20}, "c": 3}
let merged = json["merge"](base, over)
print(json["dumps"](merged))  # {"a":1,"b":{"x":10,"y":20},"c":3}

print(json["validate"]("{\"ok\":true}"))  # true
print(json["validate"]("not json"))       # false
```

---

### 8.6 `http`

```lyn
import "http"
```

All methods return a **response map**:
- `"ok"` — `true` if status 200–299
- `"status"` — HTTP status code (number)
- `"body"` — response body string
- `"headers"` — response headers map

| Method | Signature | Description |
|--------|-----------|-------------|
| `get` | `(url, headers)` | HTTP GET |
| `post` | `(url, body, headers)` | HTTP POST |
| `put` | `(url, body, headers)` | HTTP PUT |
| `patch` | `(url, body, headers)` | HTTP PATCH |
| `delete` | `(url, headers)` | HTTP DELETE |
| `head` | `(url, headers)` | HTTP HEAD |
| `request` | `(url, method, headers, body)` | Generic request |
| `bearer` | `(token)` | Build `Authorization: Bearer ...` header |
| `buildUrl` | `(base, params)` | Append query params from map |
| `download` | `(url, destPath)` | Download to file |

**Examples:**

```lyn
import "http"

# GET
let resp = http["get"]("https://httpbin.org/get", null)
if resp["ok"] {
    let data = jsonParse(resp["body"])
    print(data["url"])
}

# POST with JSON
let payload = {"user": "alice", "pass": "secret"}
let resp2 = http["post"]("https://httpbin.org/post", payload, null)
print(resp2["status"])   # 200

# Bearer auth
let headers = http["bearer"]("my-token-123")
let secure = http["get"]("https://api.example.com/profile", headers)

# Build URL with query string
let url = http["buildUrl"]("https://api.example.com/search", {
    "q": "Avelyn lang",
    "limit": 10
})
print(url)
# https://api.example.com/search?q=Avelyn%20lang&limit=10

# Download file
http["download"]("https://example.com/file.zip", "output.zip")
```

---

### 8.7 `web`

```lyn
import "web"
```

Flask/Express-style HTTP server framework.

```lyn
let app = web["app"]()

app["get"]("/", def(req) {
    return app["json"]({"message": "Hello from Evelyn!"})
})

app["post"]("/users", def(req) {
    let body = jsonParse(req["body"])
    return app["respond"](jsonStringify({"created": body["name"]}), 201)
})

app["run"](8080)
```

#### `webApp()` methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `get` | `(path, handler)` | Register GET route |
| `post` | `(path, handler)` | Register POST route |
| `put` | `(path, handler)` | Register PUT route |
| `delete` | `(path, handler)` | Register DELETE route |
| `patch` | `(path, handler)` | Register PATCH route |
| `any` | `(path, handler)` | Any HTTP method |
| `use` | `(fn)` | Add middleware |
| `json` | `(data)` | Build JSON 200 response |
| `text` | `(content)` | Build plain text 200 response |
| `html` | `(content)` | Build HTML 200 response |
| `respond` | `(body, status)` | Build custom response |
| `run` | `(port)` | Start server |

#### Global response helpers

```lyn
web["json"]({"ok": true})            # JSON 200
web["error"]("Not found", 404)       # JSON error
web["html"]("<h1>Hello</h1>")        # HTML 200
web["text"]("plain text")            # text/plain 200
web["redirect"]("https://example.com") # 302
web["parseQuery"]("name=Alice&age=30")  # {"name":"Alice","age":"30"}
web["parseBody"](req)                   # parse JSON body
```

#### Request object fields

| Field | Description |
|-------|-------------|
| `"method"` | HTTP method string |
| `"path"` | Request path |
| `"headers"` | Headers map |
| `"body"` | Raw request body |
| `"params"` | URL parameters (`:name` style) |
| `"query"` | Query string parameters |

---

### 8.8 `middleware`

```lyn
import "middleware"
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `cors` | `(origins, methods, headers)` | CORS headers middleware |
| `auth` | `(token)` | Bearer token auth |
| `rateLimit` | `(maxRequests, windowMs)` | In-memory rate limiter |
| `logger` | (no args) | Request logging |
| `jsonBody` | (no args) | Parse JSON body into `req["json"]` |
| `compose` | `(middlewares)` | Compose an array of middlewares |

**Examples:**

```lyn
import "web"
import "middleware"

let app = web["app"]()

app["use"](middleware["cors"](null, null, null))
app["use"](middleware["logger"])
app["use"](middleware["jsonBody"])

# Protected route
let protect = middleware["auth"]("secret-token")
app["get"]("/admin", protect(def(req) {
    return app["json"]({"admin": true})
}))

# Rate limit: 100 req/min
let limiter = middleware["rateLimit"](100, 60000)
app["get"]("/api/data", limiter(def(req) {
    return app["json"]({"data": "ok"})
}))

# Compose multiple
let stack = middleware["compose"]([
    middleware["cors"](null, null, null),
    middleware["logger"],
    middleware["jsonBody"]
])
app["get"]("/full", stack(def(req) {
    return app["json"]({"ok": true})
}))

app["run"](3000)
```

---

### 8.9 `async`

```lyn
import "async"
```

Cooperative event-loop and JavaScript-style Promises.

| Method | Signature | Description |
|--------|-----------|-------------|
| `delay` | `(ms, fn)` | Schedule fn after ms milliseconds |
| `loop` | `()` | Run event queue until empty |
| `promise` | `(executor)` | Create a new Promise |

**Event loop example:**

```lyn
import "async"

async["delay"](100, def() { print("100ms") })
async["delay"](50,  def() { print("50ms") })

print("Starting loop")
async["loop"]()
# prints: "50ms" then "100ms"
```

**Promise example:**

```lyn
import "async"

let p = async["promise"](def(resolve, reject) {
    let result = computeSomething()
    if not isNull(result) {
        resolve(result)
    } else {
        reject("Computation failed")
    }
})

p["then"](def(val) {
    print("Result: " + toString(val))
    return val * 2
})["catch"](def(err) {
    print("Error: " + err)
})

async["loop"]()
```

---

### 8.10 `datetime`

```lyn
import "datetime"
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `now` | `()` | Current Unix timestamp (ms) |
| `format` | `(ts, fmt)` | Format timestamp |
| `parse` | `(s, fmt)` | Parse date string → timestamp |
| `add` | `(ts, amount, unit)` | Add time (unit: `"days"`, `"hours"`, `"minutes"`, `"seconds"`) |
| `diff` | `(ts1, ts2, unit)` | Difference in given unit |
| `fromParts` | `(year, month, day)` | Build timestamp from date components |
| `startOfDay` | `(ts)` | Timestamp at 00:00:00 |
| `endOfDay` | `(ts)` | Timestamp at 23:59:59 |
| `isAfter` | `(ts1, ts2)` | ts1 > ts2? |
| `isBefore` | `(ts1, ts2)` | ts1 < ts2? |
| `isToday` | `(ts)` | Same day as now? |
| `toISO` | `(ts)` | ISO 8601 string |
| `toDate` | `(ts)` | `yyyy-MM-dd` string |
| `toTime` | `(ts)` | `HH:mm:ss` string |
| `relative` | `(ts)` | Human-relative string ("5 minutes ago") |

**Examples:**

```lyn
import "datetime"

let now = datetime["now"]()
print(datetime["toISO"](now))    # 2026-05-27T00:00:00Z
print(datetime["toDate"](now))   # 2026-05-27
print(datetime["toTime"](now))   # 14:30:00

let ts = datetime["parse"]("2026-01-15", "yyyy-MM-dd")
print(datetime["toDate"](ts))    # 2026-01-15

let tomorrow = datetime["add"](now, 1, "days")
let nextHour = datetime["add"](now, 1, "hours")
let diff = datetime["diff"](ts, now, "days")
print(diff)   # ~130

let birthday = datetime["fromParts"](2000, 6, 15)
print(datetime["toDate"](birthday))   # 2000-06-15

let fiveMinAgo = datetime["add"](now, -300000, "ms")
print(datetime["relative"](fiveMinAgo))   # 5 minutes ago
```

---

### 8.11 `random`

```lyn
import "random"
```

All functions use OS-level CSPRNG (cryptographically secure).

| Method | Signature | Description |
|--------|-----------|-------------|
| `random` | `()` | Float in `[0.0, 1.0)` |
| `randint` | `(a, b)` | Integer in `[a, b]` inclusive |
| `uniform` | `(a, b)` | Float in `[a, b]` |
| `choice` | `(arr)` | Random element |
| `shuffle` | `(arr)` | Shuffled copy |
| `sample` | `(arr, k)` | k unique random elements |

**Examples:**

```lyn
import "random"

print(random["random"]())           # e.g. 0.7392...
print(random["randint"](1, 6))      # dice roll
print(random["uniform"](0.0, 1.5))  # float in [0.0, 1.5]
print(random["choice"](["rock","paper","scissors"]))

let hand = random["sample"](["A","K","Q","J","10"], 3)
print(hand)   # 3 unique random cards
```

---

### 8.12 `re` (Regex)

```lyn
import "re"
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `search` | `(pattern, text)` | True if pattern found anywhere |
| `match` | `(pattern, text)` | True if pattern at start |
| `fullMatch` | `(pattern, text)` | True if entire text matches |
| `findAll` | `(pattern, text)` | All matching substrings |
| `groups` | `(pattern, text)` | Capture groups |
| `sub` | `(pattern, repl, text)` | Replace matches |
| `split` | `(pattern, text)` | Split by matches |
| `escape` | `(text)` | Escape special regex chars |

**Examples:**

```lyn
import "re"

print(re["search"]("\\d+", "abc 123"))        # true
print(re["match"]("\\w+", "hello world"))     # true
print(re["fullMatch"]("[a-z]+", "hello"))     # true
print(re["fullMatch"]("[a-z]+", "hello!"))    # false

let matches = re["findAll"]("\\d+", "a1 b22 c333")
print(matches)   # [1, 22, 333]

let groups = re["groups"]("(\\w+)@(\\w+)", "alice@example")
print(groups)    # [alice, example]

let clean = re["sub"]("\\s+", " ", "too   many   spaces")
print(clean)     # too many spaces

let parts = re["split"](",\\s*", "a, b,  c,d")
print(parts)     # [a, b, c, d]

let safe = re["escape"]("a+b*c")
print(safe)      # a\+b\*c
```

---

### 8.13 `collections`

```lyn
import "collections"
```

#### Stack (LIFO)

```lyn
let s = Stack()
s["push"](1)
s["push"](2)
s["push"](3)
print(s["peek"]())    # 3
print(s["pop"]())     # 3
print(s["size"]())    # 2
print(s["isEmpty"]()) # false
print(s["toArray"]()) # [1, 2]
```

| Method | Description |
|--------|-------------|
| `push(x)` | Add to top |
| `pop()` | Remove & return top |
| `peek()` | View top |
| `isEmpty()` | Empty check |
| `size()` | Element count |
| `toArray()` | Array copy |

#### Queue (FIFO)

```lyn
let q = Queue()
q["enqueue"]("task1")
q["enqueue"]("task2")
print(q["front"]())    # task1
print(q["dequeue"]())  # task1
print(q["size"]())     # 1
```

| Method | Description |
|--------|-------------|
| `enqueue(x)` | Add to back |
| `dequeue()` | Remove & return front |
| `front()` | View front |
| `isEmpty()` | Empty check |
| `size()` | Element count |
| `toArray()` | Array copy |

#### Set (unique collection)

```lyn
let s = Set(["a", "b", "c", "a"])  # deduped on init
s["add"]("d")
s["add"]("b")              # no-op
print(s["has"]("b"))       # true
s["remove"]("b")
print(s["size"]())         # 3
print(s["toArray"]())      # [a, c, d]
```

| Method | Description |
|--------|-------------|
| `add(x)` | Add (no-op if present) |
| `remove(x)` | Remove element |
| `has(x)` | Membership test |
| `size()` | Element count |
| `toArray()` | Array copy |

---

### 8.14 `functional`

```lyn
import "functional"
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `partial` | `(func, presetArgs)` | Partial application |
| `memoize` | `(func)` | Cache results (single-arg) |
| `pipe` | `(val, fns)` | Pipe value left-to-right through functions |
| `compose` | `(fns)` | Compose functions right-to-left |

**Examples:**

```lyn
import "functional"

# Partial application
let add = def(a, b) => a + b
let add10 = functional["partial"](add, [10])
print(add10(5))    # 15
print(add10(20))   # 30

# Memoize expensive function
def fib(n) {
    if n <= 1 { return n }
    return fib(n-1) + fib(n-2)
}
let fastFib = functional["memoize"](fib)
print(fastFib(10))   # 55

# Pipe: left-to-right
let result = functional["pipe"](
    "  hello world  ",
    [
        def(s) => stringTrim(s),
        def(s) => stringToUpper(s),
        def(s) => string["reverse"](s)
    ]
)
print(result)   # DLROW OLLEH

# Compose: right-to-left
let transform = functional["compose"]([
    def(s) => stringToUpper(s),
    def(s) => stringTrim(s)
])
print(transform("  hello  "))  # HELLO
```

---

### 8.15 `logger`

```lyn
import "logger"
```

Leveled, structured logger.

| Level | Value | Constant |
|-------|-------|----------|
| DEBUG | 0 | `LEVEL_DEBUG` |
| INFO  | 1 | `LEVEL_INFO`  |
| WARN  | 2 | `LEVEL_WARN`  |
| ERROR | 3 | `LEVEL_ERROR` |

| Method | Signature | Description |
|--------|-----------|-------------|
| `debug` | `(msg)` | Log DEBUG |
| `info` | `(msg)` | Log INFO |
| `warn` | `(msg)` | Log WARN |
| `error` | `(msg)` | Log ERROR |
| `ctx` | `(msg, context)` | Log INFO with context map |
| `setLevel` | `(lvl)` | Minimum level filter |
| `setFormat` | `(fmt)` | `"text"` or `"json"` |
| `setPrefix` | `(prefix)` | Add prefix to all lines |
| `child` | `(prefix)` | Child logger with prefix |

**Examples:**

```lyn
import "logger"

logger["info"]("Server started")
# [2026-05-27 14:30:00] [INFO] Server started

logger["warn"]("High memory usage")
logger["error"]("DB connection failed")

logger["ctx"]("Request", {"method":"GET","path":"/api"})
# [2026-05-27 14:30:00] [INFO] Request | {"method":"GET","path":"/api"}

logger["setFormat"]("json")
logger["info"]("JSON mode")
# {"ts":"2026-05-27 14:30:00","level":"INFO","msg":"JSON mode"}

logger["setLevel"](LEVEL_WARN)
logger["debug"]("hidden")  # filtered out
logger["warn"]("visible")  # shown
```

---

### 8.16 `validate`

```lyn
import "validate"
```

Input validation — returns `null` on success, error string on failure.

| Method | Signature | Description |
|--------|-----------|-------------|
| `required` | `(value, field)` | Must not be null or empty |
| `email` | `(value, field)` | Must be valid email |
| `minLen` | `(value, min, field)` | String length >= min |
| `maxLen` | `(value, max, field)` | String length <= max |
| `min` | `(value, min, field)` | Number >= min |
| `max` | `(value, max, field)` | Number <= max |
| `enum` | `(value, options, field)` | Must be one of options |
| `pattern` | `(value, pattern, field)` | Must match regex |
| `url` | `(value, field)` | Must be valid URL |
| `integer` | `(value, field)` | Must be integer |
| `schema` | `(data, schema)` | Validate map against schema |
| `ok` | `(errors)` | True if error array is empty |

**Examples:**

```lyn
import "validate"

print(validate["required"](null, "username"))
# 'username' is required

print(validate["email"]("bad-email", "email"))
# 'email' must be a valid email

print(validate["minLen"]("hi", 5, "password"))
# 'password' must be at least 5 characters

# Schema validation
let schema = {
    "name":  {"required": true, "minLen": 2, "maxLen": 50},
    "email": {"required": true, "email": true},
    "age":   {"min": 0, "max": 150},
    "role":  {"enum": ["user", "admin"]}
}

let data = {
    "name": "A",         # too short
    "email": "invalid",  # bad email
    "role": "superadmin" # not in enum
}

let errors = validate["schema"](data, schema)
print(validate["ok"](errors))   # false
for e in errors { print(e) }
```

---

### 8.17 `test`

```lyn
import "test"
```

pytest / Jest-style testing framework.

| Function | Signature | Description |
|----------|-----------|-------------|
| `describe` | `(suiteName, fn)` | Group tests in a suite |
| `it` | `(name, fn)` | A single test case |
| `expect` | `(actual)` | Create assertion object |
| `summary` | `()` | Print results |

#### Assertions on `expect(actual)`

| Method | Description |
|--------|-------------|
| `.toBe(expected)` | Deep equality |
| `.toEqual(expected)` | Deep equality (with JSON diff) |
| `.toBeNull()` | Must be `null` |
| `.toBeTrue()` | Must be exactly `true` |
| `.toBeFalse()` | Must be exactly `false` |
| `.toContain(item)` | Array/string contains item |
| `.toThrow(fn)` | Function must throw |
| `.toBeGreaterThan(n)` | actual > n |
| `.toBeLessThan(n)` | actual < n |
| `.toHaveLength(n)` | Array or string length equals n |

**Example:**

```lyn
import "test"
import "string"

describe("String module", def() {
    it("reverses correctly", def() {
        expect(string["reverse"]("hello"))["toBe"]("olleh")
    })
    it("uppercases correctly", def() {
        expect(string["upper"]("hello"))["toBe"]("HELLO")
    })
    it("trims whitespace", def() {
        expect(string["trim"]("  hi  "))["toBe"]("hi")
    })
})

describe("Math operations", def() {
    it("2 + 2 = 4", def() {
        expect(2 + 2)["toBe"](4)
    })
    it("floor works", def() {
        expect(mathFloor(3.9))["toBe"](3)
    })
})

describe("Error handling", def() {
    it("catches thrown errors", def() {
        expect(null)["toThrow"](def() { throw "oops" })
    })
})

testSummary()
```

Output:
```
[String module]
  ✓ reverses correctly
  ✓ uppercases correctly
  ✓ trims whitespace

[Math operations]
  ✓ 2 + 2 = 4
  ✓ floor works

[Error handling]
  ✓ catches thrown errors

Test Results: 6/6 passed
All tests passed!
```

---

### 8.18 `cli`

```lyn
import "cli"
```

Command-line argument parser.

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(name, version, description)` | Create parser |
| `addFlag` | `(parser, name, alias, description)` | Boolean flag |
| `addOption` | `(parser, name, alias, default, description)` | Value option |
| `printHelp` | `(parser)` | Print help text |
| `parse` | `(parser, argv)` | Parse args, return `{flags, options, args}` |

Built-in: `--help` / `-h` always triggers help and exits.

**Example:**

```lyn
import "cli"
import "sys"

let parser = cli["new"]("mytool", "1.0.0", "A sample CLI tool")
cli["addFlag"](parser, "--verbose", "-v", "Enable verbose output")
cli["addFlag"](parser, "--dry-run", "-d", "Dry run mode")
cli["addOption"](parser, "--output", "-o", "output.txt", "Output file")
cli["addOption"](parser, "--count",  "-n", "10",         "Item count")

let result = cli["parse"](parser, sys["argv"])

let verbose = result["flags"]["verbose"]
let output  = result["options"]["output"]
let files   = result["args"]

if verbose {
    print("Verbose mode: output=" + output)
}
for file in files {
    print("Processing: " + file)
}
```

Run: `avelyn tool.lyn --verbose -o result.txt file1.txt file2.txt`

---

### 8.19 `term`

```lyn
import "term"
```

ANSI terminal styling.

| Method | Description |
|--------|-------------|
| `format(code, text)` | Wrap text in ANSI code |
| `print(code, text)` | Print styled text |
| `red(text)` | Red text |
| `green(text)` | Green text |
| `yellow(text)` | Yellow text |
| `blue(text)` | Blue text |
| `cyan(text)` | Cyan text |
| `bold(text)` | Bold text |
| `printRed(text)` | Print red |
| `printGreen(text)` | Print green |
| `printYellow(text)` | Print yellow |
| `printBlue(text)` | Print blue |
| `printCyan(text)` | Print cyan |

Available codes: `reset`, `bold`, `dim`, `underline`, `blink`, `reverse`, `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `bgBlack`, `bgRed`, `bgGreen`, `bgYellow`, `bgBlue`, `bgMagenta`, `bgCyan`, `bgWhite`.

**Examples:**

```lyn
import "term"

print(term["red"]("Error!"))
print(term["green"]("Success!"))
print(term["yellow"]("Warning"))
print(term["bold"]("Important"))
print(term["format"]("bgBlue", "  Banner  "))
print(term["format"]("underline", "Underlined text"))

term["printRed"]("Fatal error occurred")
term["printGreen"]("All tests passed!")
```

---

### 8.20 `prompt`

```lyn
import "prompt"
```

Interactive CLI prompts.

| Method | Signature | Description |
|--------|-----------|-------------|
| `askInput` | `(question, defaultVal)` | Text input with optional default |
| `askConfirm` | `(question, defaultVal)` | Yes/No prompt |
| `askSelect` | `(question, options)` | Numbered selection list |

**Examples:**

```lyn
import "prompt"

let name = prompt["askInput"]("What is your name?", "World")
print("Hello, " + name + "!")

let proceed = prompt["askConfirm"]("Continue?", true)
if proceed {
    print("Continuing...")
}

let color = prompt["askSelect"]("Choose a color", ["Red", "Green", "Blue"])
print("You chose: " + color)
```

---

### 8.21 `template`

```lyn
import "template"
```

String templating with `{{variable}}` placeholders.

| Method | Signature | Description |
|--------|-----------|-------------|
| `render` | `(tmpl, vars)` | Replace `{{key}}` with values |
| `renderFile` | `(path, vars)` | Render a template file |
| `renderIf` | `(tmpl, vars)` | Handle `{{#if key}}...{{/if}}` blocks |
| `escape` | `(s)` | HTML-escape a string |
| `htmlTable` | `(rows, columns)` | Build HTML table from array of maps |

**Examples:**

```lyn
import "template"

let tmpl = "Hello, {{name}}! You are {{age}} years old."
print(template["render"](tmpl, {"name": "Alice", "age": 30}))
# Hello, Alice! You are 30 years old.

let html = "Welcome! {{#if premium}}You are Premium.{{/if}}"
print(template["renderIf"](html, {"premium": true}))
# Welcome! You are Premium.

print(template["escape"]("<script>alert('xss')</script>"))
# &lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;

let rows = [
    {"name": "Alice", "score": 95},
    {"name": "Bob",   "score": 87},
]
print(template["htmlTable"](rows, ["name", "score"]))
# <table><thead>...</thead><tbody>...</tbody></table>
```

---

### 8.22 `config`

```lyn
import "config"
```

Configuration management: env var → JSON file → default fallback.

| Method | Signature | Description |
|--------|-----------|-------------|
| `load` | `(filePath)` | Load JSON config file |
| `get` | `(cfg, key, default)` | Get value (dot-path, env first) |
| `getInt` | `(cfg, key, default)` | Get as integer |
| `getBool` | `(cfg, key, default)` | Get as boolean |
| `required` | `(cfg, key)` | Get or throw if missing |

**Example:**

```json
// config.json
{
  "db": { "host": "localhost", "port": 5432 },
  "debug": false
}
```

```lyn
import "config"

let cfg = config["load"]("config.json")

let host  = config["get"](cfg, "db.host", "localhost")
let port  = config["getInt"](cfg, "db.port", 5432)
let debug = config["getBool"](cfg, "debug", false)

# Throws if missing
let apiKey = config["required"](cfg, "api.key")

print("Connecting to " + host + ":" + numToString(port))
```

> Environment variable `DB_HOST` (uppercased, dots → underscores) overrides `db.host`.

---

### 8.23 `uuid`

```lyn
import "uuid"
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `uuid4` | `()` | Generate random UUID v4 |
| `isValid` | `(s)` | True if string is valid UUID v4 |

```lyn
import "uuid"

let id = uuid["uuid4"]()
print(id)                           # f47ac10b-58cc-4372-a567-0e02b2c3d479
print(uuid["isValid"](id))          # true
print(uuid["isValid"]("not-uuid"))  # false
```

---

### 8.24 `secrets`

```lyn
import "secrets"
```

OS CSPRNG-backed token generation.

| Method | Signature | Description |
|--------|-----------|-------------|
| `token_bytes` | `(n)` | n random bytes as hex string |
| `token_hex` | `(n)` | Alias for token_bytes |
| `token_urlsafe` | `(n)` | URL-safe base64 token |
| `choice` | `(arr)` | Cryptographically secure random choice |

```lyn
import "secrets"

let session = secrets["token_urlsafe"](32)
print(session)   # e.g. "eW91cl9zZWNyZXRfdG9rZW4..."

let hex = secrets["token_hex"](16)
print(hex)       # 32 hex characters

let otp = secrets["choice"](["0","1","2","3","4","5","6","7","8","9"])
```

---

### 8.25 `lzw`

```lyn
import "lzw"
```

Pure Evelyn Lempel-Ziv-Welch compression.

| Method | Signature | Description |
|--------|-----------|-------------|
| `compress` | `(s)` | Compress string → array of codes |
| `decompress` | `(arr)` | Decompress codes → original string |

```lyn
import "lzw"

let original = "ABABABABABABABAB"
let compressed = lzw["compress"](original)
print(compressed)   # [65, 66, 256, 258, 260, 262, 264]

let restored = lzw["decompress"](compressed)
print(restored == original)   # true
```

---

### 8.26 `pathlib`

```lyn
import "pathlib"
```

Object-oriented path management.

```lyn
let p = pathlib["Path"]("/home/user/docs/report.pdf")
```

| Field / Method | Description |
|----------------|-------------|
| `str` | The path string |
| `join(subPath)` | Return new Path joined with subPath |
| `basename()` | Filename component |
| `dirname()` | Parent directory as new Path |
| `ext()` | Extension (e.g. `.pdf`) |
| `exists()` | File exists? |
| `isDir()` | Is a directory? |
| `read()` | Read file contents |
| `write(data)` | Write data to file |

```lyn
import "pathlib"

let p = pathlib["Path"]("/home/user/docs/report.pdf")
print(p["str"])               # /home/user/docs/report.pdf
print(p["basename"]())        # report.pdf
print(p["ext"]())             # .pdf
print(p["dirname"]()["str"])  # /home/user/docs

let child = p["dirname"]()["join"]("notes.txt")
print(child["str"])   # /home/user/docs/notes.txt

let readme = pathlib["Path"]("README.md")
if readme["exists"]() {
    print(readme["read"]())
}
```

---

### 8.27 `sys` & `os`

```lyn
import "sys"
import "os"
```

#### `sys`

| Field / Method | Description |
|----------------|-------------|
| `argv` | Command-line arguments array |
| `version` | Evelyn version (`"1.5.0"`) |
| `exit(code)` | Exit process |
| `stdout.write(s)` | Write to stdout |
| `stderr.write(s)` | Write to stderr |

```lyn
import "sys"

print(sys["version"])    # 1.5.0
print(sys["argv"])       # args

if arrayLen(sys["argv"]) < 2 {
    sys["stderr"]["write"]("Missing argument")
    sys["exit"](1)
}
```

#### `os`

| Method | Description |
|--------|-------------|
| `path.exists(p)` | File exists |
| `path.basename(p)` | Filename |
| `path.ext(p)` | Extension |
| `path.join(parts)` | Join path |
| `system(cmd, args)` | Execute command |
| `environ(key)` | Read env variable |
| `listdir(p)` | List directory |

```lyn
import "os"

print(os["environ"]("HOME"))
let files = os["listdir"](".")
for f in files { print(f) }
os["system"]("echo", ["Hello!"])
```

---

### 8.28 `hashlib`, `base64` & `hex`

```lyn
import "hashlib"
import "base64"
import "hex"
```

#### `hashlib`

| Method | Description |
|--------|-------------|
| `md5(s)` | MD5 digest as hex string |
| `sha1(s)` | SHA-1 digest |
| `sha256(s)` | SHA-256 digest |

```lyn
import "hashlib"

print(hashlib["md5"]("hello"))
# 5d41402abc4b2a76b9719d911017c592

print(hashlib["sha256"]("hello"))
# 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
```

#### `base64`

```lyn
import "base64"

let enc = base64["b64encode"]("Hello, World!")
print(enc)                          # SGVsbG8sIFdvcmxkIQ==
print(base64["b64decode"](enc))     # Hello, World!
```

#### `hex`

```lyn
import "hex"

let h = hex["encode"]("hello")
print(h)                    # 68656c6c6f
print(hex["decode"](h))     # hello
```

---

### 8.29 `dotenv`

```lyn
import "dotenv"
```

Load `.env` files into the environment.

| Method | Signature | Description |
|--------|-----------|-------------|
| `load` | `(filePath)` | Parse & load `.env` file |
| `get` | `(key, default)` | Get loaded env value |

**`.env` file:**
```
DATABASE_URL=postgres://localhost/mydb
SECRET_KEY=my-secret
DEBUG=true
```

```lyn
import "dotenv"

dotenv["load"](".env")
print(dotenv["get"]("DATABASE_URL", "sqlite://local.db"))
print(dotenv["get"]("DEBUG", "false"))
```

---

### 8.30 `glob` & `shutil`

```lyn
import "glob"
import "shutil"
```

#### `glob` — file pattern matching

```lyn
import "glob"

let sylFiles = glob["glob"]("src/**/*.lyn")
for f in sylFiles { print(f) }
```

#### `shutil` — file operations

| Method | Signature | Description |
|--------|-----------|-------------|
| `copy` | `(src, dst)` | Copy file |
| `move` | `(src, dst)` | Move or rename |
| `remove` | `(path)` | Delete file |
| `rmdir` | `(path)` | Remove directory |

```lyn
import "shutil"

shutil["copy"]("config.json", "config.json.bak")
shutil["move"]("old.txt", "new.txt")
shutil["remove"]("temp.txt")
```

---

### 8.31 `platform` & `time`

```lyn
import "platform"
import "time"
```

#### `platform`

| Field | Description |
|-------|-------------|
| `os` | OS name: `"windows"`, `"macos"`, `"linux"` |
| `arch` | CPU architecture string |
| `isWindows` | True on Windows |
| `isMac` | True on macOS |
| `isLinux` | True on Linux |

```lyn
import "platform"

print(platform["os"])   # macos
if platform["isWindows"] { print("Windows!") }
```

#### `time`

| Method | Signature | Description |
|--------|-----------|-------------|
| `now` | `()` | Unix timestamp in seconds |
| `ms` | `()` | Current time in milliseconds |
| `sleep` | `(ms)` | Sleep milliseconds |
| `format` | `(ts, fmt)` | Format timestamp |

```lyn
import "time"

let start = time["ms"]()
# ... work ...
let elapsed = time["ms"]() - start
print("Elapsed: " + numToString(elapsed) + "ms")

time["sleep"](500)   # pause 500ms
```

---

## 9. Complete Example Programs

### Example 1: Simple REST API

```lyn
import "web"
import "json"
import "validate"
import "uuid"

let app = web["app"]()
var _users = []

app["get"]("/users", def(req) {
    return app["json"](_users)
})

app["get"]("/users/:id", def(req) {
    let id = req["params"]["id"]
    for u in _users {
        if u["id"] == id {
            return app["json"](u)
        }
    }
    return app["respond"](jsonStringify({"error": "Not found"}), 404)
})

app["post"]("/users", def(req) {
    let body = jsonParse(req["body"])
    let schema = {
        "name":  {"required": true, "minLen": 2},
        "email": {"required": true, "email": true}
    }
    let errors = validate["schema"](body, schema)
    if not validate["ok"](errors) {
        return app["respond"](jsonStringify({"errors": errors}), 422)
    }
    let user = {
        "id":    uuid["uuid4"](),
        "name":  body["name"],
        "email": body["email"]
    }
    arrayAppend(_users, user)
    return app["respond"](jsonStringify(user), 201)
})

app["run"](8080)
```

---

### Example 2: File Processing Tool

```lyn
import "io"
import "string"
import "json"
import "array"

def readCSV(filePath) {
    let content = io["read"](filePath)
    if isNull(content) { throw "Cannot read: " + filePath }

    let lines = string["split"](stringTrim(content), "\n")
    let headers = string["split"](lines[0], ",")
    var records = []

    for i in 1..arrayLen(lines):
        let line = stringTrim(lines[i])
        if stringLen(line) == 0 { continue }
        let values = string["split"](line, ",")
        var record = {}
        for j in 0..arrayLen(headers):
            mapSet(record, headers[j], values[j])
        arrayAppend(records, record)

    return records
}

let data = readCSV("students.csv")
let scores = arrayMap(data, def(r) => stringToNum(r["score"]))
print("Total students: " + numToString(arrayLen(data)))
print("Average score:  " + numToString(array["avg"](scores)))
print("Top score:      " + numToString(array["max"](scores)))

io["write"]("summary.json", json["pretty"](data))
print("Written to summary.json")
```

---

### Example 3: CLI Tool

```lyn
import "cli"
import "sys"
import "io"
import "string"
import "term"

let parser = cli["new"]("wc", "1.0.0", "Count words or lines in files")
cli["addFlag"](parser, "--lines", "-l", "Count lines (default: words)")
cli["addFlag"](parser, "--verbose", "-v", "Show details per file")
cli["addOption"](parser, "--output", "-o", "", "Save results to file")

let result = cli["parse"](parser, sys["argv"])
let countLines = result["flags"]["lines"]
let verbose    = result["flags"]["verbose"]
let outputFile = result["options"]["output"]
let files      = result["args"]

if arrayLen(files) == 0 {
    term["printRed"]("Error: No files provided")
    cli["printHelp"](parser)
    sys["exit"](1)
}

var totalCount = 0
var output = []

for file in files {
    let content = io["read"](file)
    if isNull(content) {
        term["printYellow"]("Warning: Cannot read " + file)
        continue
    }
    var count = 0
    if countLines {
        count = arrayLen(string["split"](content, "\n"))
    } else {
        count = arrayLen(string["split"](stringTrim(content), " "))
    }
    totalCount += count
    let line = file + ": " + numToString(count)
    arrayAppend(output, line)
    if verbose { term["printCyan"](line) }
}

print(term["bold"]("Total: " + numToString(totalCount)))

if stringLen(outputFile) > 0 {
    io["writeLines"](outputFile, output)
    print("Results saved to " + outputFile)
}
```

---

### Example 4: Async Event Queue

```lyn
import "async"
import "logger"
import "datetime"

logger["setPrefix"]("scheduler")

def scheduleTask(name, delayMs) {
    logger["info"]("Scheduling '" + name + "' in " + numToString(delayMs) + "ms")
    async["delay"](delayMs, def() {
        logger["info"]("Running: " + name)
    })
}

scheduleTask("cleanup",    1000)
scheduleTask("backup",     500)
scheduleTask("heartbeat",  200)
scheduleTask("report",     800)

logger["info"]("Starting event loop")
async["loop"]()
logger["info"]("All tasks completed!")
```

---

### Example 5: Data Validation Pipeline

```lyn
import "validate"
import "json"
import "logger"
import "functional"

def buildValidator(schema) {
    return def(data) {
        let errors = validate["schema"](data, schema)
        return {
            "ok":     validate["ok"](errors),
            "errors": errors,
            "data":   data
        }
    }
}

let userSchema = {
    "username": {"required": true, "minLen": 3, "maxLen": 20, "pattern": "^[a-zA-Z0-9_]+$"},
    "email":    {"required": true, "email": true},
    "age":      {"required": false, "min": 13, "max": 120, "integer": true},
    "role":     {"enum": ["user", "admin", "moderator"]}
}

let validateUser = buildValidator(userSchema)

let users = [
    {"username": "alice_99", "email": "alice@example.com", "age": 25, "role": "admin"},
    {"username": "b",        "email": "bad-email",          "age": 5,  "role": "god"},
    {"username": "charlie",  "email": "charlie@test.com",              "role": "user"},
]

for user in users {
    let result = validateUser(user)
    if result["ok"] {
        logger["info"]("Valid user: " + user["username"])
    } else {
        logger["warn"]("Invalid: " + user["username"])
        for e in result["errors"] { logger["warn"]("  - " + e) }
    }
}
```

---

*Evelyn (Avelyn) v2.5.7 — Official Language & Compiler Reference*
