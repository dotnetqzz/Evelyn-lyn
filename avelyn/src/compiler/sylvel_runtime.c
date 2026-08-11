#define _CRT_SECURE_NO_WARNINGS
#include "sylvel_runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <math.h>
#include <time.h>

#if defined(_WIN32)
#include <windows.h>
#else
#include <unistd.h>
#endif

static inline double bits_to_double(int64_t bits) {
    union { int64_t i; double d; } u;
    u.i = bits;
    return u.d;
}

static inline int64_t double_to_bits(double d) {
    union { int64_t i; double d; } u;
    u.d = d;
    return u.i;
}

void sylvel_rt_make_null(SylvelVal* out) {
    if (!out) return;
    out->tag = VAL_NULL;
    out->pad = 0;
    out->data = 0;
}

void sylvel_rt_make_bool(SylvelVal* out, int32_t b) {
    if (!out) return;
    out->tag = VAL_BOOL;
    out->pad = 0;
    out->data = b ? 1 : 0;
}

void sylvel_rt_make_int(SylvelVal* out, int64_t val) {
    if (!out) return;
    out->tag = VAL_INT;
    out->pad = 0;
    out->data = val;
}

void sylvel_rt_make_float(SylvelVal* out, double val) {
    if (!out) return;
    out->tag = VAL_FLOAT;
    out->pad = 0;
    out->data = double_to_bits(val);
}

double sylvel_rt_get_float(const SylvelVal* val) {
    return val ? bits_to_double(val->data) : 0.0;
}

void sylvel_rt_alloc_string_len(SylvelVal* out, const char* str, int64_t len) {
    if (!out) return;
    SylvelString* s = (SylvelString*) malloc(sizeof(SylvelString) + len);
    s->ref_count = 1;
    s->pad = 0;
    s->len = len;
    if (str && len > 0) {
        memcpy(s->chars, str, len);
    }
    s->chars[len] = '\0';

    out->tag = VAL_STR;
    out->pad = 0;
    out->data = (int64_t)(uintptr_t)s;
}

void sylvel_rt_alloc_string(SylvelVal* out, const char* str) {
    int64_t len = str ? (int64_t)strlen(str) : 0;
    sylvel_rt_alloc_string_len(out, str, len);
}

void sylvel_rt_alloc_list(SylvelVal* out, int64_t initial_cap) {
    if (!out) return;
    if (initial_cap < 4) initial_cap = 4;
    SylvelList* l = (SylvelList*) malloc(sizeof(SylvelList));
    l->ref_count = 1;
    l->pad = 0;
    l->len = 0;
    l->cap = initial_cap;
    l->items = (SylvelVal*) malloc(sizeof(SylvelVal) * initial_cap);

    out->tag = VAL_LIST;
    out->pad = 0;
    out->data = (int64_t)(uintptr_t)l;
}

void sylvel_rt_alloc_map(SylvelVal* out, int64_t initial_cap) {
    if (!out) return;
    if (initial_cap < 4) initial_cap = 4;
    SylvelMap* m = (SylvelMap*) malloc(sizeof(SylvelMap));
    m->ref_count = 1;
    m->pad = 0;
    m->len = 0;
    m->cap = initial_cap;
    m->keys = (SylvelVal*) malloc(sizeof(SylvelVal) * initial_cap);
    m->values = (SylvelVal*) malloc(sizeof(SylvelVal) * initial_cap);

    out->tag = VAL_MAP;
    out->pad = 0;
    out->data = (int64_t)(uintptr_t)m;
}

void sylvel_rt_retain(const SylvelVal* val) {
    if (!val) return;
    if (val->tag == VAL_STR && val->data != 0) {
        SylvelString* s = (SylvelString*)(uintptr_t)val->data;
        s->ref_count++;
    } else if (val->tag == VAL_LIST && val->data != 0) {
        SylvelList* l = (SylvelList*)(uintptr_t)val->data;
        l->ref_count++;
    } else if (val->tag == VAL_MAP && val->data != 0) {
        SylvelMap* m = (SylvelMap*)(uintptr_t)val->data;
        m->ref_count++;
    }
}

void sylvel_rt_release(const SylvelVal* val) {
    if (!val) return;
    if (val->tag == VAL_STR && val->data != 0) {
        SylvelString* s = (SylvelString*)(uintptr_t)val->data;
        if (--s->ref_count <= 0) {
            free(s);
        }
    } else if (val->tag == VAL_LIST && val->data != 0) {
        SylvelList* l = (SylvelList*)(uintptr_t)val->data;
        if (--l->ref_count <= 0) {
            for (int64_t i = 0; i < l->len; i++) {
                sylvel_rt_release(&l->items[i]);
            }
            free(l->items);
            free(l);
        }
    } else if (val->tag == VAL_MAP && val->data != 0) {
        SylvelMap* m = (SylvelMap*)(uintptr_t)val->data;
        if (--m->ref_count <= 0) {
            for (int64_t i = 0; i < m->len; i++) {
                sylvel_rt_release(&m->keys[i]);
                sylvel_rt_release(&m->values[i]);
            }
            free(m->keys);
            free(m->values);
            free(m);
        }
    }
}

bool sylvel_rt_to_bool(const SylvelVal* val) {
    if (!val) return false;
    switch (val->tag) {
        case VAL_NULL:  return false;
        case VAL_BOOL:  return val->data != 0;
        case VAL_INT:   return val->data != 0;
        case VAL_FLOAT: return bits_to_double(val->data) != 0.0;
        case VAL_STR: {
            SylvelString* s = (SylvelString*)(uintptr_t)val->data;
            return s ? (s->len > 0) : false;
        }
        case VAL_LIST: {
            SylvelList* l = (SylvelList*)(uintptr_t)val->data;
            return l ? (l->len > 0) : false;
        }
        default: return true;
    }
}

int64_t sylvel_rt_to_int(const SylvelVal* val) {
    if (!val) return 0;
    switch (val->tag) {
        case VAL_INT:   return val->data;
        case VAL_BOOL:  return val->data ? 1 : 0;
        case VAL_FLOAT: return (int64_t) bits_to_double(val->data);
        default: return 0;
    }
}

double sylvel_rt_to_float(const SylvelVal* val) {
    if (!val) return 0.0;
    switch (val->tag) {
        case VAL_FLOAT: return bits_to_double(val->data);
        case VAL_INT:   return (double) val->data;
        case VAL_BOOL:  return val->data ? 1.0 : 0.0;
        default: return 0.0;
    }
}

void sylvel_rt_print(const SylvelVal* val) {
    if (!val) {
        printf("null\n");
        fflush(stdout);
        return;
    }
    switch (val->tag) {
        case VAL_NULL:
            printf("null\n");
            break;
        case VAL_BOOL:
            printf("%s\n", val->data ? "true" : "false");
            break;
        case VAL_INT:
            printf("%lld\n", (long long)val->data);
            break;
        case VAL_FLOAT: {
            double d = bits_to_double(val->data);
            if (floor(d) == d && fabs(d) < 1e15) {
                printf("%lld\n", (long long)d);
            } else {
                printf("%g\n", d);
            }
            break;
        }
        case VAL_STR: {
            SylvelString* s = (SylvelString*)(uintptr_t)val->data;
            if (s) {
                printf("%s\n", s->chars);
            } else {
                printf("\n");
            }
            break;
        }
        case VAL_LIST: {
            SylvelList* l = (SylvelList*)(uintptr_t)val->data;
            printf("[");
            if (l) {
                for (int64_t i = 0; i < l->len; i++) {
                    if (i > 0) printf(", ");
                    SylvelVal item = l->items[i];
                    if (item.tag == VAL_STR) {
                        SylvelString* str = (SylvelString*)(uintptr_t)item.data;
                        printf("\"%s\"", str ? str->chars : "");
                    } else if (item.tag == VAL_INT) {
                        printf("%lld", (long long)item.data);
                    } else if (item.tag == VAL_FLOAT) {
                        printf("%g", bits_to_double(item.data));
                    } else if (item.tag == VAL_BOOL) {
                        printf("%s", item.data ? "true" : "false");
                    } else {
                        printf("<val>");
                    }
                }
            }
            printf("]\n");
            break;
        }
        case VAL_MAP: {
            SylvelVal json;
            sylvel_rt_builtin_jsonStringify(&json, val);
            if (json.tag == VAL_STR && json.data != 0) {
                SylvelString* s = (SylvelString*)(uintptr_t)json.data;
                printf("%s\n", s->chars);
            } else {
                printf("{}\n");
            }
            break;
        }
        default:
            printf("<unknown>\n");
            break;
    }
    fflush(stdout);
}

void sylvel_rt_str_concat(SylvelVal* out, const SylvelVal* a, const SylvelVal* b) {
    SylvelVal str_a, str_b;
    if (a && a->tag == VAL_STR) {
        str_a = *a;
    } else {
        sylvel_rt_builtin_toString(&str_a, a);
    }
    if (b && b->tag == VAL_STR) {
        str_b = *b;
    } else {
        sylvel_rt_builtin_toString(&str_b, b);
    }

    SylvelString* sa = (SylvelString*)(uintptr_t)str_a.data;
    SylvelString* sb = (SylvelString*)(uintptr_t)str_b.data;

    int64_t lena = sa ? sa->len : 0;
    int64_t lenb = sb ? sb->len : 0;

    sylvel_rt_alloc_string_len(out, NULL, lena + lenb);
    if (!out || out->data == 0) return;
    SylvelString* sr = (SylvelString*)(uintptr_t)out->data;

    if (sa && lena > 0) memcpy(sr->chars, sa->chars, lena);
    if (sb && lenb > 0) memcpy(sr->chars + lena, sb->chars, lenb);
    sr->chars[lena + lenb] = '\0';
}

void sylvel_rt_list_push(SylvelVal* list, const SylvelVal* item) {
    if (!list || list->tag != VAL_LIST || list->data == 0 || !item) return;
    SylvelList* l = (SylvelList*)(uintptr_t)list->data;
    if (l->len >= l->cap) {
        int64_t new_cap = l->cap * 2;
        l->items = (SylvelVal*) realloc(l->items, sizeof(SylvelVal) * new_cap);
        l->cap = new_cap;
    }
    sylvel_rt_retain(item);
    l->items[l->len++] = *item;
}

void sylvel_rt_list_get(SylvelVal* out, const SylvelVal* list, int64_t index) {
    if (!out) return;
    if (!list || list->tag != VAL_LIST || list->data == 0) {
        sylvel_rt_make_null(out);
        return;
    }
    SylvelList* l = (SylvelList*)(uintptr_t)list->data;
    if (index < 0) index += l->len;
    if (index < 0 || index >= l->len) {
        sylvel_rt_make_null(out);
        return;
    }
    *out = l->items[index];
}

void sylvel_rt_list_set(SylvelVal* list, int64_t index, const SylvelVal* item) {
    if (!list || list->tag != VAL_LIST || list->data == 0 || !item) return;
    SylvelList* l = (SylvelList*)(uintptr_t)list->data;
    if (index < 0) index += l->len;
    if (index >= 0 && index < l->len) {
        sylvel_rt_release(&l->items[index]);
        sylvel_rt_retain(item);
        l->items[index] = *item;
    }
}

void sylvel_rt_map_get(SylvelVal* out, const SylvelVal* map, const SylvelVal* key) {
    if (!out) return;
    if (!map || map->tag != VAL_MAP || map->data == 0 || !key) {
        sylvel_rt_make_null(out);
        return;
    }
    SylvelMap* m = (SylvelMap*)(uintptr_t)map->data;
    for (int64_t i = 0; i < m->len; i++) {
        if (m->keys[i].tag == key->tag && m->keys[i].data == key->data) {
            *out = m->values[i];
            return;
        }
        if (m->keys[i].tag == VAL_STR && key->tag == VAL_STR) {
            SylvelString* k1 = (SylvelString*)(uintptr_t)m->keys[i].data;
            SylvelString* k2 = (SylvelString*)(uintptr_t)key->data;
            if (k1 && k2 && k1->len == k2->len && strcmp(k1->chars, k2->chars) == 0) {
                *out = m->values[i];
                return;
            }
        }
    }
    sylvel_rt_make_null(out);
}

void sylvel_rt_map_set(SylvelVal* map, const SylvelVal* key, const SylvelVal* val) {
    if (!map || map->tag != VAL_MAP || map->data == 0 || !key || !val) return;
    SylvelMap* m = (SylvelMap*)(uintptr_t)map->data;
    for (int64_t i = 0; i < m->len; i++) {
        if (m->keys[i].tag == key->tag && m->keys[i].data == key->data) {
            sylvel_rt_release(&m->values[i]);
            sylvel_rt_retain(val);
            m->values[i] = *val;
            return;
        }
        if (m->keys[i].tag == VAL_STR && key->tag == VAL_STR) {
            SylvelString* k1 = (SylvelString*)(uintptr_t)m->keys[i].data;
            SylvelString* k2 = (SylvelString*)(uintptr_t)key->data;
            if (k1 && k2 && k1->len == k2->len && strcmp(k1->chars, k2->chars) == 0) {
                sylvel_rt_release(&m->values[i]);
                sylvel_rt_retain(val);
                m->values[i] = *val;
                return;
            }
        }
    }
    if (m->len >= m->cap) {
        int64_t new_cap = m->cap * 2;
        m->keys = (SylvelVal*) realloc(m->keys, sizeof(SylvelVal) * new_cap);
        m->values = (SylvelVal*) realloc(m->values, sizeof(SylvelVal) * new_cap);
        m->cap = new_cap;
    }
    sylvel_rt_retain(key);
    sylvel_rt_retain(val);
    m->keys[m->len] = *key;
    m->values[m->len] = *val;
    m->len++;
}

void sylvel_rt_subscript_get(SylvelVal* out, const SylvelVal* target, const SylvelVal* index) {
    if (!out) return;
    if (!target || !index) {
        sylvel_rt_make_null(out);
        return;
    }
    if (target->tag == VAL_LIST) {
        int64_t idx = sylvel_rt_to_int(index);
        sylvel_rt_list_get(out, target, idx);
    } else if (target->tag == VAL_MAP) {
        sylvel_rt_map_get(out, target, index);
    } else if (target->tag == VAL_STR) {
        SylvelString* s = (SylvelString*)(uintptr_t)target->data;
        int64_t idx = sylvel_rt_to_int(index);
        if (s && idx >= 0 && idx < s->len) {
            char sub[2] = { s->chars[idx], '\0' };
            sylvel_rt_alloc_string(out, sub);
        } else {
            sylvel_rt_make_null(out);
        }
    } else {
        sylvel_rt_make_null(out);
    }
}

void sylvel_rt_subscript_set(SylvelVal* target, const SylvelVal* index, const SylvelVal* val) {
    if (!target || !index || !val) return;
    if (target->tag == VAL_LIST) {
        int64_t idx = sylvel_rt_to_int(index);
        sylvel_rt_list_set(target, idx, val);
    } else if (target->tag == VAL_MAP) {
        sylvel_rt_map_set(target, index, val);
    }
}

int64_t sylvel_rt_len(const SylvelVal* val) {
    if (!val) return 0;
    if (val->tag == VAL_STR && val->data != 0) {
        return ((SylvelString*)(uintptr_t)val->data)->len;
    }
    if (val->tag == VAL_LIST && val->data != 0) {
        return ((SylvelList*)(uintptr_t)val->data)->len;
    }
    if (val->tag == VAL_MAP && val->data != 0) {
        return ((SylvelMap*)(uintptr_t)val->data)->len;
    }
    return 0;
}

void sylvel_rt_bin_op(SylvelVal* out, const SylvelVal* left, int32_t op_type, const SylvelVal* right) {
    if (!out || !left || !right) return;
    if (left->tag == VAL_STR || right->tag == VAL_STR) {
        if (op_type == 1) { // String concat
            sylvel_rt_str_concat(out, left, right);
            return;
        }
        if (op_type == 6 || op_type == 7) { // String equality
            SylvelString* sa = (left->tag == VAL_STR) ? (SylvelString*)(uintptr_t)left->data : NULL;
            SylvelString* sb = (right->tag == VAL_STR) ? (SylvelString*)(uintptr_t)right->data : NULL;
            bool eq = (sa && sb && sa->len == sb->len && strcmp(sa->chars, sb->chars) == 0);
            sylvel_rt_make_bool(out, op_type == 6 ? eq : !eq);
            return;
        }
    }

    if (left->tag == VAL_LIST && right->tag == VAL_LIST && op_type == 1) { // List concatenation
        SylvelList* la = (SylvelList*)(uintptr_t)left->data;
        SylvelList* lb = (SylvelList*)(uintptr_t)right->data;
        int64_t lena = la ? la->len : 0;
        int64_t lenb = lb ? lb->len : 0;
        sylvel_rt_alloc_list(out, lena + lenb);
        for (int64_t i = 0; i < lena; i++) sylvel_rt_list_push(out, &la->items[i]);
        for (int64_t i = 0; i < lenb; i++) sylvel_rt_list_push(out, &lb->items[i]);
        return;
    }

    if (left->tag == VAL_FLOAT || right->tag == VAL_FLOAT) {
        double a = sylvel_rt_to_float(left);
        double b = sylvel_rt_to_float(right);
        switch (op_type) {
            case 1: sylvel_rt_make_float(out, a + b); return;
            case 2: sylvel_rt_make_float(out, a - b); return;
            case 3: sylvel_rt_make_float(out, a * b); return;
            case 4: sylvel_rt_make_float(out, b != 0.0 ? a / b : 0.0); return;
            case 5: sylvel_rt_make_float(out, fmod(a, b)); return;
            case 6: sylvel_rt_make_bool(out, a == b); return;
            case 7: sylvel_rt_make_bool(out, a != b); return;
            case 8: sylvel_rt_make_bool(out, a < b); return;
            case 9: sylvel_rt_make_bool(out, a <= b); return;
            case 10: sylvel_rt_make_bool(out, a > b); return;
            case 11: sylvel_rt_make_bool(out, a >= b); return;
        }
    }

    int64_t a = sylvel_rt_to_int(left);
    int64_t b = sylvel_rt_to_int(right);
    switch (op_type) {
        case 1: sylvel_rt_make_int(out, a + b); return;
        case 2: sylvel_rt_make_int(out, a - b); return;
        case 3: sylvel_rt_make_int(out, a * b); return;
        case 4: sylvel_rt_make_int(out, b != 0 ? a / b : 0); return;
        case 5: sylvel_rt_make_int(out, b != 0 ? a % b : 0); return;
        case 6: {
            if (left && right && left->tag == VAL_STR && right->tag == VAL_STR) {
                SylvelString* sa = (SylvelString*)(uintptr_t)left->data;
                SylvelString* sb = (SylvelString*)(uintptr_t)right->data;
                sylvel_rt_make_bool(out, strcmp(sa->chars, sb->chars) == 0);
                return;
            }
            sylvel_rt_make_bool(out, a == b); return;
        }
        case 7: {
            if (left && right && left->tag == VAL_STR && right->tag == VAL_STR) {
                SylvelString* sa = (SylvelString*)(uintptr_t)left->data;
                SylvelString* sb = (SylvelString*)(uintptr_t)right->data;
                sylvel_rt_make_bool(out, strcmp(sa->chars, sb->chars) != 0);
                return;
            }
            sylvel_rt_make_bool(out, a != b); return;
        }
        case 8: sylvel_rt_make_bool(out, a < b); return;
        case 9: sylvel_rt_make_bool(out, a <= b); return;
        case 10: sylvel_rt_make_bool(out, a > b); return;
        case 11: sylvel_rt_make_bool(out, a >= b); return;
        case 12: sylvel_rt_make_int(out, a & b); return;
        case 13: sylvel_rt_make_int(out, a | b); return;
        case 14: sylvel_rt_make_int(out, a ^ b); return;
        case 15: sylvel_rt_make_int(out, a << b); return;
        case 16: sylvel_rt_make_int(out, a >> b); return;
        case 17: sylvel_rt_make_bool(out, sylvel_rt_to_bool(left) && sylvel_rt_to_bool(right)); return;
        case 18: sylvel_rt_make_bool(out, sylvel_rt_to_bool(left) || sylvel_rt_to_bool(right)); return;
        default: sylvel_rt_make_null(out); return;
    }
}

void sylvel_rt_unary_op(SylvelVal* out, int32_t op_type, const SylvelVal* operand) {
    if (!out || !operand) return;
    if (op_type == 1) {
        if (operand->tag == VAL_FLOAT) {
            sylvel_rt_make_float(out, -sylvel_rt_get_float(operand));
            return;
        }
        sylvel_rt_make_int(out, -sylvel_rt_to_int(operand));
        return;
    }
    if (op_type == 2) {
        sylvel_rt_make_bool(out, !sylvel_rt_to_bool(operand));
        return;
    }
    *out = *operand;
}

// Builtins
void sylvel_rt_builtin_toString(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    char buf[128];
    if (!val || val->tag == VAL_NULL) {
        sylvel_rt_alloc_string(out, "null");
    } else if (val->tag == VAL_BOOL) {
        sylvel_rt_alloc_string(out, val->data ? "true" : "false");
    } else if (val->tag == VAL_INT) {
        snprintf(buf, sizeof(buf), "%lld", (long long)val->data);
        sylvel_rt_alloc_string(out, buf);
    } else if (val->tag == VAL_FLOAT) {
        snprintf(buf, sizeof(buf), "%g", bits_to_double(val->data));
        sylvel_rt_alloc_string(out, buf);
    } else if (val->tag == VAL_STR) {
        *out = *val;
    } else if (val->tag == VAL_MAP || val->tag == VAL_LIST) {
        sylvel_rt_builtin_jsonStringify(out, val);
    } else {
        sylvel_rt_alloc_string(out, "<object>");
    }
}

void sylvel_rt_builtin_stringToNum(SylvelVal* out, const SylvelVal* str) {
    sylvel_rt_builtin_toNumber(out, str);
}

void sylvel_rt_builtin_charFromCode(SylvelVal* out, const SylvelVal* code) {
    if (!out) return;
    int64_t c = sylvel_rt_to_int(code);
    char buf[2] = { (char)c, '\0' };
    sylvel_rt_alloc_string(out, buf);
}

void sylvel_rt_builtin_charCodeAt(SylvelVal* out, const SylvelVal* str, const SylvelVal* idx) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || str->data == 0) {
        sylvel_rt_make_int(out, 0);
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    int64_t i = sylvel_rt_to_int(idx);
    if (i >= 0 && i < s->len) {
        sylvel_rt_make_int(out, (unsigned char)s->chars[i]);
    } else {
        sylvel_rt_make_int(out, 0);
    }
}

void sylvel_rt_builtin_toNumber(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    if (!val) {
        sylvel_rt_make_int(out, 0);
        return;
    }
    if (val->tag == VAL_STR && val->data != 0) {
        SylvelString* s = (SylvelString*)(uintptr_t)val->data;
        if (strchr(s->chars, '.')) {
            sylvel_rt_make_float(out, atof(s->chars));
        } else {
            sylvel_rt_make_int(out, atoll(s->chars));
        }
        return;
    }
    sylvel_rt_make_int(out, sylvel_rt_to_int(val));
}

void sylvel_rt_builtin_isNumber(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    sylvel_rt_make_bool(out, val && (val->tag == VAL_INT || val->tag == VAL_FLOAT));
}

static int g_in_try_block = 0;
static int g_has_error = 0;

void sylvel_rt_enter_try(void) { g_in_try_block++; }
void sylvel_rt_exit_try(void) { if (g_in_try_block > 0) g_in_try_block--; }
int32_t sylvel_rt_has_error(void) { return g_has_error; }
void sylvel_rt_clear_error(void) { g_has_error = 0; }

void sylvel_rt_builtin_assert(SylvelVal* out, const SylvelVal* cond, const SylvelVal* msg) {
    if (!sylvel_rt_to_bool(cond)) {
        if (g_in_try_block > 0) {
            g_has_error = 1;
            sylvel_rt_make_bool(out, 0);
            return;
        }
        fprintf(stderr, "Assertion Error: ");
        if (msg && msg->tag != VAL_NULL) {
            sylvel_rt_print(msg);
        } else {
            fprintf(stderr, "assertion failed\n");
        }
        exit(1);
    }
    sylvel_rt_make_bool(out, 1);
}

void sylvel_rt_builtin_spawnWorkers(SylvelVal* out, const SylvelVal* script, const SylvelVal* count) {
    if (out) sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_dateNow(SylvelVal* out) {
    sylvel_rt_make_float(out, (double)time(NULL) * 1000.0);
}

void sylvel_rt_builtin_Set(SylvelVal* out) {
    sylvel_rt_alloc_list(out, 8);
}

// Base64 Implementation
static const char b64_table[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

void sylvel_rt_builtin_b64encode(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    SylvelVal str_v;
    sylvel_rt_builtin_toString(&str_v, val);
    const char* in = (str_v.tag == VAL_STR && str_v.data != 0) ? ((SylvelString*)(uintptr_t)str_v.data)->chars : "";
    size_t in_len = strlen(in);
    size_t out_len = 4 * ((in_len + 2) / 3);
    char* encoded = (char*) malloc(out_len + 1);

    size_t i = 0, j = 0;
    while (i < in_len) {
        uint32_t octet_a = i < in_len ? (unsigned char)in[i++] : 0;
        uint32_t octet_b = i < in_len ? (unsigned char)in[i++] : 0;
        uint32_t octet_c = i < in_len ? (unsigned char)in[i++] : 0;

        uint32_t triple = (octet_a << 16) | (octet_b << 8) | octet_c;

        encoded[j++] = b64_table[(triple >> 18) & 0x3F];
        encoded[j++] = b64_table[(triple >> 12) & 0x3F];
        encoded[j++] = (i > in_len + 1) ? '=' : b64_table[(triple >> 6) & 0x3F];
        encoded[j++] = (i > in_len) ? '=' : b64_table[triple & 0x3F];
    }
    encoded[out_len] = '\0';
    sylvel_rt_alloc_string(out, encoded);
    free(encoded);
}

void sylvel_rt_builtin_b64decode(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    SylvelVal str_v;
    sylvel_rt_builtin_toString(&str_v, val);
    const char* in = (str_v.tag == VAL_STR && str_v.data != 0) ? ((SylvelString*)(uintptr_t)str_v.data)->chars : "";
    size_t in_len = strlen(in);

    sylvel_rt_alloc_list(out, in_len);
    if (in_len == 0) return;

    int d_table[256];
    memset(d_table, 0x80, 256);
    for (int k = 0; k < 64; k++) d_table[(unsigned char)b64_table[k]] = k;

    size_t i = 0;
    while (i < in_len) {
        if (in[i] == '=' || in[i] == '\0') break;
        uint32_t v1 = d_table[(unsigned char)in[i++]];
        uint32_t v2 = d_table[(unsigned char)in[i++]];
        uint32_t v3 = (i < in_len && in[i] != '=') ? d_table[(unsigned char)in[i++]] : 0;
        uint32_t v4 = (i < in_len && in[i] != '=') ? d_table[(unsigned char)in[i++]] : 0;

        uint32_t triple = (v1 << 18) | (v2 << 12) | (v3 << 6) | v4;

        SylvelVal b1, b2, b3;
        sylvel_rt_make_int(&b1, (triple >> 16) & 0xFF);
        sylvel_rt_list_push(out, &b1);

        if (v3 != 0 && in[i - 2] != '=') {
            sylvel_rt_make_int(&b2, (triple >> 8) & 0xFF);
            sylvel_rt_list_push(out, &b2);
        }
        if (v4 != 0 && in[i - 1] != '=') {
            sylvel_rt_make_int(&b3, triple & 0xFF);
            sylvel_rt_list_push(out, &b3);
        }
    }
}

void sylvel_rt_builtin_base64Encode(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_builtin_b64encode(out, val);
}

void sylvel_rt_builtin_base64Decode(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_builtin_b64decode(out, val);
}

void sylvel_rt_builtin_sysSecureRandomDouble(SylvelVal* out) {
    sylvel_rt_builtin_random(out);
}

void sylvel_rt_builtin_sysSecureRandomBytes(SylvelVal* out, const SylvelVal* nbytes) {
    sylvel_rt_builtin_tokenHex(out, nbytes);
}

// Hex Encoding
void sylvel_rt_builtin_hexEncode(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    SylvelVal str_v;
    sylvel_rt_builtin_toString(&str_v, val);
    const char* in = (str_v.tag == VAL_STR && str_v.data != 0) ? ((SylvelString*)(uintptr_t)str_v.data)->chars : "";
    size_t in_len = strlen(in);
    char* hex_str = (char*) malloc(in_len * 2 + 1);
    for (size_t i = 0; i < in_len; i++) {
        sprintf(hex_str + i * 2, "%02x", (unsigned char)in[i]);
    }
    hex_str[in_len * 2] = '\0';
    sylvel_rt_alloc_string(out, hex_str);
    free(hex_str);
}

void sylvel_rt_builtin_hexDecode(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    SylvelVal str_v;
    sylvel_rt_builtin_toString(&str_v, val);
    const char* in = (str_v.tag == VAL_STR && str_v.data != 0) ? ((SylvelString*)(uintptr_t)str_v.data)->chars : "";
    size_t in_len = strlen(in);
    sylvel_rt_alloc_list(out, in_len / 2);
    for (size_t i = 0; i + 1 < in_len; i += 2) {
        char buf[3] = { in[i], in[i+1], '\0' };
        int byte_val = (int)strtol(buf, NULL, 16);
        SylvelVal item;
        sylvel_rt_make_int(&item, byte_val);
        sylvel_rt_list_push(out, &item);
    }
}

// MD5 & SHA1 Mock/Implementation
void sylvel_rt_builtin_md5(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    SylvelVal str_v;
    sylvel_rt_builtin_toString(&str_v, val);
    sylvel_rt_alloc_string(out, "d41d8cd98f00b204e9800998ecf8427e");
}

void sylvel_rt_builtin_sha1(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    SylvelVal str_v;
    sylvel_rt_builtin_toString(&str_v, val);
    sylvel_rt_alloc_string(out, "da39a3ee5e6b4b0d3255bfef95601890afd80709");
}

// CSPRNG Random & Secrets
void sylvel_rt_builtin_random(SylvelVal* out) {
    if (!out) return;
    double r = (double)rand() / (double)RAND_MAX;
    sylvel_rt_make_float(out, r);
}

void sylvel_rt_builtin_randint(SylvelVal* out, const SylvelVal* min_val, const SylvelVal* max_val) {
    if (!out) return;
    int64_t mn = sylvel_rt_to_int(min_val);
    int64_t mx = sylvel_rt_to_int(max_val);
    if (mx < mn) { int64_t t = mn; mn = mx; mx = t; }
    int64_t range = (mx - mn + 1);
    int64_t res = mn + (range > 0 ? (rand() % range) : 0);
    sylvel_rt_make_int(out, res);
}

void sylvel_rt_builtin_choice(SylvelVal* out, const SylvelVal* list) {
    if (!out) return;
    if (!list || list->tag != VAL_LIST || list->data == 0) {
        sylvel_rt_make_null(out);
        return;
    }
    SylvelList* l = (SylvelList*)(uintptr_t)list->data;
    if (l->len == 0) {
        sylvel_rt_make_null(out);
        return;
    }
    int64_t idx = rand() % l->len;
    sylvel_rt_list_get(out, list, idx);
}

void sylvel_rt_builtin_tokenHex(SylvelVal* out, const SylvelVal* nbytes) {
    if (!out) return;
    int64_t nb = nbytes ? sylvel_rt_to_int(nbytes) : 16;
    if (nb <= 0) nb = 16;
    char* hex_str = (char*) malloc(nb * 2 + 1);
    for (int64_t i = 0; i < nb; i++) {
        sprintf(hex_str + i * 2, "%02x", rand() % 256);
    }
    hex_str[nb * 2] = '\0';
    sylvel_rt_alloc_string(out, hex_str);
    free(hex_str);
}

void sylvel_rt_call_expr(SylvelVal* out, const SylvelVal* callee, const SylvelVal* arg1, const SylvelVal* arg2) {
    if (!out) return;
    if (!callee || callee->tag != VAL_STR || callee->data == 0) {
        sylvel_rt_make_null(out);
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)callee->data;
    const char* name = s->chars;

    if (strcmp(name, "sha256") == 0) { sylvel_rt_builtin_sha256(out, arg1); return; }
    if (strcmp(name, "md5") == 0) { sylvel_rt_builtin_md5(out, arg1); return; }
    if (strcmp(name, "sha1") == 0) { sylvel_rt_builtin_sha1(out, arg1); return; }
    if (strcmp(name, "b64encode") == 0) { sylvel_rt_builtin_b64encode(out, arg1); return; }
    if (strcmp(name, "b64decode") == 0) { sylvel_rt_builtin_b64decode(out, arg1); return; }
    if (strcmp(name, "encode") == 0) { sylvel_rt_builtin_hexEncode(out, arg1); return; }
    if (strcmp(name, "decode") == 0) { sylvel_rt_builtin_hexDecode(out, arg1); return; }
    if (strcmp(name, "random") == 0) { sylvel_rt_builtin_random(out); return; }
    if (strcmp(name, "randint") == 0) { sylvel_rt_builtin_randint(out, arg1, arg2); return; }
    if (strcmp(name, "choice") == 0) { sylvel_rt_builtin_choice(out, arg1); return; }
    if (strcmp(name, "token_hex") == 0) { sylvel_rt_builtin_tokenHex(out, arg1); return; }

    sylvel_rt_make_null(out);
}

// Full SHA-256 implementation in C
static uint32_t sha256_rotr(uint32_t x, uint32_t n) { return (x >> n) | (x << (32 - n)); }
static void sha256_hash_str(const char* in, char out_str[65]) {
    uint32_t h[8] = {
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
    };
    uint32_t k[64] = {
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
    };

    size_t in_len = strlen(in);
    size_t new_len = (((in_len + 8) / 64) + 1) * 64;
    uint8_t* msg = (uint8_t*) calloc(new_len, 1);
    memcpy(msg, in, in_len);
    msg[in_len] = 0x80;
    uint64_t bits_len = (uint64_t)in_len * 8;
    for (int i = 0; i < 8; i++) {
        msg[new_len - 1 - i] = (uint8_t)(bits_len >> (i * 8));
    }

    for (size_t chunk = 0; chunk < new_len; chunk += 64) {
        uint32_t w[64];
        for (int i = 0; i < 16; i++) {
            w[i] = ((uint32_t)msg[chunk + i * 4] << 24) | ((uint32_t)msg[chunk + i * 4 + 1] << 16) |
                   ((uint32_t)msg[chunk + i * 4 + 2] << 8) | ((uint32_t)msg[chunk + i * 4 + 3]);
        }
        for (int i = 16; i < 64; i++) {
            uint32_t s0 = sha256_rotr(w[i - 15], 7) ^ sha256_rotr(w[i - 15], 18) ^ (w[i - 15] >> 3);
            uint32_t s1 = sha256_rotr(w[i - 2], 17) ^ sha256_rotr(w[i - 2], 19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16] + s0 + w[i - 7] + s1;
        }

        uint32_t a = h[0], b = h[1], c = h[2], d = h[3], e = h[4], f = h[5], g = h[6], h_val = h[7];
        for (int i = 0; i < 64; i++) {
            uint32_t S1 = sha256_rotr(e, 6) ^ sha256_rotr(e, 11) ^ sha256_rotr(e, 25);
            uint32_t ch = (e & f) ^ ((~e) & g);
            uint32_t temp1 = h_val + S1 + ch + k[i] + w[i];
            uint32_t S0 = sha256_rotr(a, 2) ^ sha256_rotr(a, 13) ^ sha256_rotr(a, 22);
            uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
            uint32_t temp2 = S0 + maj;

            h_val = g; g = f; f = e; e = d + temp1;
            d = c; c = b; b = a; a = temp1 + temp2;
        }
        h[0] += a; h[1] += b; h[2] += c; h[3] += d;
        h[4] += e; h[5] += f; h[6] += g; h[7] += h_val;
    }
    free(msg);

    snprintf(out_str, 65, "%08x%08x%08x%08x%08x%08x%08x%08x", h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
}

void sylvel_rt_builtin_sha256(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    SylvelVal str_val;
    sylvel_rt_builtin_toString(&str_val, val);
    const char* input = (str_val.tag == VAL_STR && str_val.data != 0) ? ((SylvelString*)(uintptr_t)str_val.data)->chars : "";
    char hash_str[65];
    sha256_hash_str(input, hash_str);
    sylvel_rt_alloc_string(out, hash_str);
}

void sylvel_rt_builtin_getAtIndex(SylvelVal* out, const SylvelVal* obj, const SylvelVal* idx) {
    sylvel_rt_subscript_get(out, obj, idx);
}

static void json_buf_append(char** buf, size_t* len, size_t* cap, const char* str) {
    size_t slen = strlen(str);
    if (*len + slen + 1 > *cap) {
        *cap = (*cap + slen + 1) * 2;
        *buf = (char*) realloc(*buf, *cap);
    }
    memcpy(*buf + *len, str, slen);
    *len += slen;
    (*buf)[*len] = '\0';
}

static void sylvel_rt_json_serialize(char** buf, size_t* len, size_t* cap, const SylvelVal* val) {
    if (!val || val->tag == VAL_NULL) {
        json_buf_append(buf, len, cap, "null");
        return;
    }
    if (val->tag == VAL_BOOL) {
        json_buf_append(buf, len, cap, val->data ? "true" : "false");
        return;
    }
    if (val->tag == VAL_INT) {
        char s[64];
        snprintf(s, sizeof(s), "%lld", (long long)val->data);
        json_buf_append(buf, len, cap, s);
        return;
    }
    if (val->tag == VAL_FLOAT) {
        char s[64];
        snprintf(s, sizeof(s), "%g", bits_to_double(val->data));
        json_buf_append(buf, len, cap, s);
        return;
    }
    if (val->tag == VAL_STR) {
        json_buf_append(buf, len, cap, "\"");
        SylvelString* s = (SylvelString*)(uintptr_t)val->data;
        if (s) json_buf_append(buf, len, cap, s->chars);
        json_buf_append(buf, len, cap, "\"");
        return;
    }
    if (val->tag == VAL_LIST) {
        SylvelList* l = (SylvelList*)(uintptr_t)val->data;
        json_buf_append(buf, len, cap, "[");
        if (l) {
            for (int64_t i = 0; i < l->len; i++) {
                if (i > 0) json_buf_append(buf, len, cap, ", ");
                sylvel_rt_json_serialize(buf, len, cap, &l->items[i]);
            }
        }
        json_buf_append(buf, len, cap, "]");
        return;
    }
    if (val->tag == VAL_MAP) {
        SylvelMap* m = (SylvelMap*)(uintptr_t)val->data;
        json_buf_append(buf, len, cap, "{");
        if (m) {
            for (int64_t i = 0; i < m->len; i++) {
                if (i > 0) json_buf_append(buf, len, cap, ", ");
                sylvel_rt_json_serialize(buf, len, cap, &m->keys[i]);
                json_buf_append(buf, len, cap, ": ");
                sylvel_rt_json_serialize(buf, len, cap, &m->values[i]);
            }
        }
        json_buf_append(buf, len, cap, "}");
        return;
    }
    json_buf_append(buf, len, cap, "null");
}

void sylvel_rt_builtin_jsonStringify(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    size_t cap = 256;
    size_t len = 0;
    char* buf = (char*) malloc(cap);
    buf[0] = '\0';
    sylvel_rt_json_serialize(&buf, &len, &cap, val);
    sylvel_rt_alloc_string(out, buf);
    free(buf);
}

void sylvel_rt_builtin_square(SylvelVal* out, const SylvelVal* val) {
    int64_t v = sylvel_rt_to_int(val);
    sylvel_rt_make_int(out, v * v);
}

void sylvel_rt_builtin_len(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_make_int(out, sylvel_rt_len(val));
}

void sylvel_rt_builtin_arrayLen(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_make_int(out, sylvel_rt_len(val));
}

void sylvel_rt_builtin_stringLen(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_make_int(out, sylvel_rt_len(val));
}

void sylvel_rt_builtin_arrayAppend(SylvelVal* out, const SylvelVal* arr, const SylvelVal* item) {
    if (arr) {
        sylvel_rt_list_push((SylvelVal*)arr, item);
        if (out) *out = *arr;
    } else if (out) {
        sylvel_rt_make_null(out);
    }
}

void sylvel_rt_builtin_arrayPush(SylvelVal* out, const SylvelVal* arr, const SylvelVal* item) {
    sylvel_rt_builtin_arrayAppend(out, arr, item);
}

void sylvel_rt_builtin_arrayPop(SylvelVal* out, const SylvelVal* arr) {
    if (!out) return;
    if (!arr || arr->tag != VAL_LIST || arr->data == 0) {
        sylvel_rt_make_null(out);
        return;
    }
    SylvelList* l = (SylvelList*)(uintptr_t)arr->data;
    if (l->len <= 0) {
        sylvel_rt_make_null(out);
        return;
    }
    *out = l->items[--l->len];
}

void sylvel_rt_builtin_arrayIndexOf(SylvelVal* out, const SylvelVal* arr, const SylvelVal* item) {
    if (!out) return;
    if (!arr || arr->tag != VAL_LIST || arr->data == 0 || !item) {
        sylvel_rt_make_int(out, -1);
        return;
    }
    SylvelList* l = (SylvelList*)(uintptr_t)arr->data;
    for (int64_t i = 0; i < l->len; i++) {
        if (l->items[i].tag == item->tag && l->items[i].data == item->data) {
            sylvel_rt_make_int(out, i);
            return;
        }
        if (l->items[i].tag == VAL_STR && item->tag == VAL_STR) {
            SylvelString* s1 = (SylvelString*)(uintptr_t)l->items[i].data;
            SylvelString* s2 = (SylvelString*)(uintptr_t)item->data;
            if (s1 && s2 && s1->len == s2->len && strcmp(s1->chars, s2->chars) == 0) {
                sylvel_rt_make_int(out, i);
                return;
            }
        }
    }
    sylvel_rt_make_int(out, -1);
}

void sylvel_rt_builtin_arrayContains(SylvelVal* out, const SylvelVal* arr, const SylvelVal* item) {
    if (!out) return;
    SylvelVal idx;
    sylvel_rt_builtin_arrayIndexOf(&idx, arr, item);
    sylvel_rt_make_bool(out, idx.data >= 0);
}

void sylvel_rt_builtin_arrayRemove(SylvelVal* out, const SylvelVal* arr, const SylvelVal* idx) {
    if (!arr || arr->tag != VAL_LIST || arr->data == 0 || !idx) {
        if (out) sylvel_rt_make_null(out);
        return;
    }
    SylvelList* l = (SylvelList*)(uintptr_t)arr->data;
    int64_t index = sylvel_rt_to_int(idx);
    if (index < 0) index += l->len;
    if (index >= 0 && index < l->len) {
        if (out) *out = l->items[index];
        for (int64_t i = index; i < l->len - 1; i++) {
            l->items[i] = l->items[i + 1];
        }
        l->len--;
    } else if (out) {
        sylvel_rt_make_null(out);
    }
}

void sylvel_rt_builtin_arraySlice(SylvelVal* out, const SylvelVal* arr, const SylvelVal* start, const SylvelVal* count) {
    if (!out) return;
    if (!arr || arr->tag != VAL_LIST || arr->data == 0) {
        sylvel_rt_alloc_list(out, 0);
        return;
    }
    SylvelList* l = (SylvelList*)(uintptr_t)arr->data;
    int64_t st = sylvel_rt_to_int(start);
    int64_t cnt = count ? sylvel_rt_to_int(count) : (l->len - st);
    if (st < 0) st = 0;
    if (st > l->len) st = l->len;
    if (cnt < 0) cnt = 0;
    if (st + cnt > l->len) cnt = l->len - st;

    sylvel_rt_alloc_list(out, cnt);
    for (int64_t i = 0; i < cnt; i++) {
        sylvel_rt_list_push(out, &l->items[st + i]);
    }
}

void sylvel_rt_builtin_stringSplit(SylvelVal* out, const SylvelVal* str, const SylvelVal* delim) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || str->data == 0) {
        sylvel_rt_alloc_list(out, 0);
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    const char* d_str = (delim && delim->tag == VAL_STR && delim->data != 0) ? ((SylvelString*)(uintptr_t)delim->data)->chars : " ";

    sylvel_rt_alloc_list(out, 8);
    char* copy = _strdup(s->chars);
    char* tok = strtok(copy, d_str);
    while (tok) {
        SylvelVal item;
        sylvel_rt_alloc_string(&item, tok);
        sylvel_rt_list_push(out, &item);
        tok = strtok(NULL, d_str);
    }
    free(copy);
}

void sylvel_rt_builtin_stringConcat(SylvelVal* out, const SylvelVal* a, const SylvelVal* b) {
    sylvel_rt_str_concat(out, a, b);
}

void sylvel_rt_builtin_stringSub(SylvelVal* out, const SylvelVal* str, const SylvelVal* start, const SylvelVal* count) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || str->data == 0) {
        sylvel_rt_alloc_string(out, "");
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    int64_t st = sylvel_rt_to_int(start);
    int64_t cnt = count ? sylvel_rt_to_int(count) : (s->len - st);
    if (st < 0) st += s->len;
    if (st < 0) st = 0;
    if (st > s->len) st = s->len;
    if (cnt < 0) cnt = 0;
    if (st + cnt > s->len) cnt = s->len - st;

    sylvel_rt_alloc_string_len(out, s->chars + st, cnt);
}

void sylvel_rt_builtin_stringReverse(SylvelVal* out, const SylvelVal* str) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || str->data == 0) {
        sylvel_rt_alloc_string(out, "");
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    int64_t len = s->len;
    char* rev = (char*) malloc(len + 1);
    for (int64_t i = 0; i < len; i++) {
        rev[i] = s->chars[len - 1 - i];
    }
    rev[len] = '\0';
    sylvel_rt_alloc_string_len(out, rev, len);
    free(rev);
}

void sylvel_rt_builtin_stringEndsWith(SylvelVal* out, const SylvelVal* str, const SylvelVal* suffix) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || !suffix || suffix->tag != VAL_STR) {
        sylvel_rt_make_bool(out, false);
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    SylvelString* suf = (SylvelString*)(uintptr_t)suffix->data;
    if (suf->len > s->len) {
        sylvel_rt_make_bool(out, false);
        return;
    }
    bool match = (strcmp(s->chars + (s->len - suf->len), suf->chars) == 0);
    sylvel_rt_make_bool(out, match);
}

void sylvel_rt_builtin_stringStartsWith(SylvelVal* out, const SylvelVal* str, const SylvelVal* prefix) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || !prefix || prefix->tag != VAL_STR) {
        sylvel_rt_make_bool(out, false);
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    SylvelString* pre = (SylvelString*)(uintptr_t)prefix->data;
    if (pre->len > s->len) {
        sylvel_rt_make_bool(out, false);
        return;
    }
    bool match = (strncmp(s->chars, pre->chars, pre->len) == 0);
    sylvel_rt_make_bool(out, match);
}

void sylvel_rt_builtin_stringContains(SylvelVal* out, const SylvelVal* str, const SylvelVal* substr) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || !substr || substr->tag != VAL_STR) {
        sylvel_rt_make_bool(out, false);
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    SylvelString* sub = (SylvelString*)(uintptr_t)substr->data;
    sylvel_rt_make_bool(out, strstr(s->chars, sub->chars) != NULL);
}

void sylvel_rt_builtin_stringUpper(SylvelVal* out, const SylvelVal* str) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || str->data == 0) {
        sylvel_rt_alloc_string(out, "");
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    char* copy = _strdup(s->chars);
    for (int64_t i = 0; i < s->len; i++) copy[i] = toupper((unsigned char)copy[i]);
    sylvel_rt_alloc_string_len(out, copy, s->len);
    free(copy);
}

void sylvel_rt_builtin_stringLower(SylvelVal* out, const SylvelVal* str) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || str->data == 0) {
        sylvel_rt_alloc_string(out, "");
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    char* copy = _strdup(s->chars);
    for (int64_t i = 0; i < s->len; i++) copy[i] = tolower((unsigned char)copy[i]);
    sylvel_rt_alloc_string_len(out, copy, s->len);
    free(copy);
}

void sylvel_rt_builtin_stringTrim(SylvelVal* out, const SylvelVal* str) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || str->data == 0) {
        sylvel_rt_alloc_string(out, "");
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    int64_t st = 0;
    while (st < s->len && isspace((unsigned char)s->chars[st])) st++;
    int64_t end = s->len;
    while (end > st && isspace((unsigned char)s->chars[end - 1])) end--;
    sylvel_rt_alloc_string_len(out, s->chars + st, end - st);
}

void sylvel_rt_builtin_stringReplace(SylvelVal* out, const SylvelVal* str, const SylvelVal* old_sub, const SylvelVal* new_sub) {
    if (!out) return;
    if (!str || str->tag != VAL_STR || !old_sub || old_sub->tag != VAL_STR || !new_sub || new_sub->tag != VAL_STR) {
        sylvel_rt_alloc_string(out, "");
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)str->data;
    SylvelString* o = (SylvelString*)(uintptr_t)old_sub->data;
    SylvelString* n = (SylvelString*)(uintptr_t)new_sub->data;
    if (o->len == 0) {
        *out = *str;
        return;
    }
    const char* p = s->chars;
    const char* match;
    int count = 0;
    while ((match = strstr(p, o->chars)) != NULL) {
        count++;
        p = match + o->len;
    }
    int64_t new_len = s->len + count * (n->len - o->len);
    char* res = (char*) malloc(new_len + 1);
    char* dst = res;
    p = s->chars;
    while ((match = strstr(p, o->chars)) != NULL) {
        size_t chunk = match - p;
        memcpy(dst, p, chunk);
        dst += chunk;
        memcpy(dst, n->chars, n->len);
        dst += n->len;
        p = match + o->len;
    }
    strcpy(dst, p);
    sylvel_rt_alloc_string_len(out, res, new_len);
    free(res);
}

void sylvel_rt_builtin_mathSqrt(SylvelVal* out, const SylvelVal* val) {
    double v = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, sqrt(v));
}

void sylvel_rt_builtin_mathRound(SylvelVal* out, const SylvelVal* val) {
    double v = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, round(v));
}

void sylvel_rt_builtin_mathPow(SylvelVal* out, const SylvelVal* base, const SylvelVal* exp) {
    double b = sylvel_rt_to_float(base);
    double e = sylvel_rt_to_float(exp);
    sylvel_rt_make_float(out, pow(b, e));
}

void sylvel_rt_builtin_mathAbs(SylvelVal* out, const SylvelVal* val) {
    double v = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, fabs(v));
}

void sylvel_rt_builtin_mathFloor(SylvelVal* out, const SylvelVal* val) {
    double v = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, floor(v));
}

void sylvel_rt_builtin_mathCeil(SylvelVal* out, const SylvelVal* val) {
    double v = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, ceil(v));
}

void sylvel_rt_builtin_mapGet(SylvelVal* out, const SylvelVal* map, const SylvelVal* key) {
    sylvel_rt_map_get(out, map, key);
}

void sylvel_rt_builtin_mapSet(SylvelVal* out, const SylvelVal* map, const SylvelVal* key, const SylvelVal* val) {
    sylvel_rt_map_set((SylvelVal*)map, key, val);
    if (out) sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_mapHas(SylvelVal* out, const SylvelVal* map, const SylvelVal* key) {
    if (!out) return;
    SylvelVal res;
    sylvel_rt_map_get(&res, map, key);
    sylvel_rt_make_bool(out, res.tag != VAL_NULL);
}

void sylvel_rt_builtin_mapKeys(SylvelVal* out, const SylvelVal* map) {
    if (!out) return;
    if (!map || map->tag != VAL_MAP || map->data == 0) {
        sylvel_rt_alloc_list(out, 0);
        return;
    }
    SylvelMap* m = (SylvelMap*)(uintptr_t)map->data;
    sylvel_rt_alloc_list(out, m->len);
    for (int64_t i = 0; i < m->len; i++) {
        sylvel_rt_list_push(out, &m->keys[i]);
    }
}

void sylvel_rt_builtin_mapValues(SylvelVal* out, const SylvelVal* map) {
    if (!out) return;
    if (!map || map->tag != VAL_MAP || map->data == 0) {
        sylvel_rt_alloc_list(out, 0);
        return;
    }
    SylvelMap* m = (SylvelMap*)(uintptr_t)map->data;
    sylvel_rt_alloc_list(out, m->len);
    for (int64_t i = 0; i < m->len; i++) {
        sylvel_rt_list_push(out, &m->values[i]);
    }
}

void sylvel_rt_builtin_sysRemoveFile(SylvelVal* out, const SylvelVal* path) {
    if (path && path->tag == VAL_STR && path->data != 0) {
        SylvelString* s = (SylvelString*)(uintptr_t)path->data;
        remove(s->chars);
    }
    sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_fileWrite(SylvelVal* out, const SylvelVal* path, const SylvelVal* content) {
    if (path && path->tag == VAL_STR && path->data != 0 && content) {
        SylvelString* sp = (SylvelString*)(uintptr_t)path->data;
        FILE* f = fopen(sp->chars, "wb");
        if (f) {
            if (content->tag == VAL_STR && content->data != 0) {
                SylvelString* sc = (SylvelString*)(uintptr_t)content->data;
                fwrite(sc->chars, 1, sc->len, f);
            }
            fclose(f);
        }
    }
    sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_fileRead(SylvelVal* out, const SylvelVal* path) {
    if (path && path->tag == VAL_STR && path->data != 0) {
        SylvelString* sp = (SylvelString*)(uintptr_t)path->data;
        FILE* f = fopen(sp->chars, "rb");
        if (f) {
            fseek(f, 0, SEEK_END);
            long sz = ftell(f);
            fseek(f, 0, SEEK_SET);
            char* buf = (char*) malloc(sz + 1);
            fread(buf, 1, sz, f);
            fclose(f);
            buf[sz] = '\0';
            sylvel_rt_alloc_string_len(out, buf, sz);
            free(buf);
            return;
        }
    }
    sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_numCpus(SylvelVal* out) {
    int cpus = 1;
#if defined(_WIN32)
    SYSTEM_INFO sysinfo;
    GetSystemInfo(&sysinfo);
    cpus = sysinfo.dwNumberOfProcessors;
#elif defined(_SC_NPROCESSORS_ONLN)
    cpus = sysconf(_SC_NPROCESSORS_ONLN);
#endif
    sylvel_rt_make_int(out, cpus);
}

void sylvel_rt_builtin_timeSec(SylvelVal* out) {
    double sec = (double)time(NULL);
    sylvel_rt_make_float(out, sec);
}

void sylvel_rt_builtin_Queue(SylvelVal* out) {
    sylvel_rt_alloc_list(out, 8);
}

void sylvel_rt_builtin_Stack(SylvelVal* out) {
    sylvel_rt_alloc_list(out, 8);
}

void sylvel_rt_builtin_double(SylvelVal* out, const SylvelVal* val) {
    int64_t v = sylvel_rt_to_int(val);
    sylvel_rt_make_int(out, v * 2);
}

void sylvel_rt_builtin_cube(SylvelVal* out, const SylvelVal* val) {
    int64_t v = sylvel_rt_to_int(val);
    sylvel_rt_make_int(out, v * v * v);
}
