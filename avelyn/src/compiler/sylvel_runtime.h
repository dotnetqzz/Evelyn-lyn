#ifndef SYLVEL_RUNTIME_H
#define SYLVEL_RUNTIME_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    VAL_NULL  = 0,
    VAL_BOOL  = 1,
    VAL_INT   = 2,
    VAL_FLOAT = 3,
    VAL_STR   = 4,
    VAL_LIST  = 5,
    VAL_MAP   = 6,
} SylvelTag;

typedef struct {
    int32_t tag;
    int32_t pad; // Alignment padding
    int64_t data;
} SylvelVal;

typedef struct {
    int32_t ref_count;
    int32_t pad;
    int64_t len;
    char chars[1];
} SylvelString;

typedef struct {
    int32_t ref_count;
    int32_t pad;
    int64_t len;
    int64_t cap;
    SylvelVal* items;
} SylvelList;

typedef struct {
    int32_t ref_count;
    int32_t pad;
    int64_t len;
    int64_t cap;
    SylvelVal* keys;
    SylvelVal* values;
} SylvelMap;

// Constructor helpers
void sylvel_rt_make_null(SylvelVal* out);
void sylvel_rt_make_bool(SylvelVal* out, int32_t b);
void sylvel_rt_make_int(SylvelVal* out, int64_t val);
void sylvel_rt_make_float(SylvelVal* out, double val);
void sylvel_rt_alloc_string(SylvelVal* out, const char* str);
void sylvel_rt_alloc_string_len(SylvelVal* out, const char* str, int64_t len);
void sylvel_rt_alloc_list(SylvelVal* out, int64_t initial_cap);
void sylvel_rt_alloc_map(SylvelVal* out, int64_t initial_cap);

// Extractors
double sylvel_rt_get_float(const SylvelVal* val);
bool sylvel_rt_to_bool(const SylvelVal* val);
int64_t sylvel_rt_to_int(const SylvelVal* val);
double sylvel_rt_to_float(const SylvelVal* val);

// Memory Management (ARC)
void sylvel_rt_retain(const SylvelVal* val);
void sylvel_rt_release(const SylvelVal* val);

// Try Catch Exception Handling
void sylvel_rt_enter_try(void);
void sylvel_rt_exit_try(void);
int32_t sylvel_rt_has_error(void);
void sylvel_rt_clear_error(void);

// Operations & Builtins
void sylvel_rt_print(const SylvelVal* val);
void sylvel_rt_str_concat(SylvelVal* out, const SylvelVal* a, const SylvelVal* b);
void sylvel_rt_list_push(SylvelVal* list, const SylvelVal* item);
void sylvel_rt_list_get(SylvelVal* out, const SylvelVal* list, int64_t index);
void sylvel_rt_list_set(SylvelVal* list, int64_t index, const SylvelVal* item);
void sylvel_rt_map_get(SylvelVal* out, const SylvelVal* map, const SylvelVal* key);
void sylvel_rt_map_set(SylvelVal* map, const SylvelVal* key, const SylvelVal* val);
void sylvel_rt_subscript_get(SylvelVal* out, const SylvelVal* target, const SylvelVal* index);
void sylvel_rt_subscript_set(SylvelVal* target, const SylvelVal* index, const SylvelVal* val);
void sylvel_rt_call_expr(SylvelVal* out, const SylvelVal* callee, const SylvelVal* arg1, const SylvelVal* arg2);
int64_t sylvel_rt_len(const SylvelVal* val);

// Operations
void sylvel_rt_bin_op(SylvelVal* out, const SylvelVal* left, int32_t op_type, const SylvelVal* right);
void sylvel_rt_unary_op(SylvelVal* out, int32_t op_type, const SylvelVal* operand);

// Standard Builtin Functions for Native Binaries
void sylvel_rt_builtin_toString(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_toNumber(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_stringToNum(SylvelVal* out, const SylvelVal* str);
void sylvel_rt_builtin_charFromCode(SylvelVal* out, const SylvelVal* code);
void sylvel_rt_builtin_charCodeAt(SylvelVal* out, const SylvelVal* str, const SylvelVal* idx);
void sylvel_rt_builtin_isNumber(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_assert(SylvelVal* out, const SylvelVal* cond, const SylvelVal* msg);
void sylvel_rt_builtin_spawnWorkers(SylvelVal* out, const SylvelVal* script, const SylvelVal* count);
void sylvel_rt_builtin_dateNow(SylvelVal* out);
void sylvel_rt_builtin_Set(SylvelVal* out);
void sylvel_rt_builtin_sha256(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_md5(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_sha1(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_b64encode(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_b64decode(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_base64Encode(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_base64Decode(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_hexEncode(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_hexDecode(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_random(SylvelVal* out);
void sylvel_rt_builtin_randint(SylvelVal* out, const SylvelVal* min_val, const SylvelVal* max_val);
void sylvel_rt_builtin_choice(SylvelVal* out, const SylvelVal* list);
void sylvel_rt_builtin_tokenHex(SylvelVal* out, const SylvelVal* nbytes);
void sylvel_rt_builtin_sysSecureRandomDouble(SylvelVal* out);
void sylvel_rt_builtin_sysSecureRandomBytes(SylvelVal* out, const SylvelVal* nbytes);
void sylvel_rt_builtin_getAtIndex(SylvelVal* out, const SylvelVal* obj, const SylvelVal* idx);
void sylvel_rt_builtin_jsonStringify(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_square(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_len(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_arrayLen(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_stringLen(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_arrayAppend(SylvelVal* out, const SylvelVal* arr, const SylvelVal* item);
void sylvel_rt_builtin_arrayPush(SylvelVal* out, const SylvelVal* arr, const SylvelVal* item);
void sylvel_rt_builtin_arrayPop(SylvelVal* out, const SylvelVal* arr);
void sylvel_rt_builtin_arrayIndexOf(SylvelVal* out, const SylvelVal* arr, const SylvelVal* item);
void sylvel_rt_builtin_arrayContains(SylvelVal* out, const SylvelVal* arr, const SylvelVal* item);
void sylvel_rt_builtin_arrayRemove(SylvelVal* out, const SylvelVal* arr, const SylvelVal* idx);
void sylvel_rt_builtin_arraySlice(SylvelVal* out, const SylvelVal* arr, const SylvelVal* start, const SylvelVal* count);
void sylvel_rt_builtin_stringSplit(SylvelVal* out, const SylvelVal* str, const SylvelVal* delim);
void sylvel_rt_builtin_stringConcat(SylvelVal* out, const SylvelVal* a, const SylvelVal* b);
void sylvel_rt_builtin_stringSub(SylvelVal* out, const SylvelVal* str, const SylvelVal* start, const SylvelVal* count);
void sylvel_rt_builtin_stringReverse(SylvelVal* out, const SylvelVal* str);
void sylvel_rt_builtin_stringEndsWith(SylvelVal* out, const SylvelVal* str, const SylvelVal* suffix);
void sylvel_rt_builtin_stringStartsWith(SylvelVal* out, const SylvelVal* str, const SylvelVal* prefix);
void sylvel_rt_builtin_stringContains(SylvelVal* out, const SylvelVal* str, const SylvelVal* substr);
void sylvel_rt_builtin_stringUpper(SylvelVal* out, const SylvelVal* str);
void sylvel_rt_builtin_stringLower(SylvelVal* out, const SylvelVal* str);
void sylvel_rt_builtin_stringTrim(SylvelVal* out, const SylvelVal* str);
void sylvel_rt_builtin_stringReplace(SylvelVal* out, const SylvelVal* str, const SylvelVal* old_sub, const SylvelVal* new_sub);
void sylvel_rt_builtin_mathSqrt(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_mathRound(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_mathPow(SylvelVal* out, const SylvelVal* base, const SylvelVal* exp);
void sylvel_rt_builtin_mathAbs(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_mathFloor(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_mathCeil(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_mapGet(SylvelVal* out, const SylvelVal* map, const SylvelVal* key);
void sylvel_rt_builtin_mapSet(SylvelVal* out, const SylvelVal* map, const SylvelVal* key, const SylvelVal* val);
void sylvel_rt_builtin_mapHas(SylvelVal* out, const SylvelVal* map, const SylvelVal* key);
void sylvel_rt_builtin_mapKeys(SylvelVal* out, const SylvelVal* map);
void sylvel_rt_builtin_mapValues(SylvelVal* out, const SylvelVal* map);
void sylvel_rt_builtin_sysRemoveFile(SylvelVal* out, const SylvelVal* path);
void sylvel_rt_builtin_fileWrite(SylvelVal* out, const SylvelVal* path, const SylvelVal* content);
void sylvel_rt_builtin_fileRead(SylvelVal* out, const SylvelVal* path);
void sylvel_rt_builtin_numCpus(SylvelVal* out);
void sylvel_rt_builtin_timeSec(SylvelVal* out);
void sylvel_rt_builtin_Queue(SylvelVal* out);
void sylvel_rt_builtin_Stack(SylvelVal* out);
void sylvel_rt_builtin_double(SylvelVal* out, const SylvelVal* val);
void sylvel_rt_builtin_cube(SylvelVal* out, const SylvelVal* val);

#ifdef __cplusplus
}
#endif

#endif // SYLVEL_RUNTIME_H
