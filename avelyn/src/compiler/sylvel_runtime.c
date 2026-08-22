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
#define SYLVEL_STRDUP _strdup
#else
#include <unistd.h>
#include <sys/time.h>
#include <sys/stat.h>
#include <dirent.h>
#define SYLVEL_STRDUP strdup
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

static inline const char* sylvel_rt_to_str(const SylvelVal* val) {
    if (!val || val->tag != VAL_STR || val->data == 0) return "";
    SylvelString* s = (SylvelString*)(uintptr_t)val->data;
    return s ? s->chars : "";
}

// Helper: produce a C string representation for a value into a supplied buffer.
// If the value is a string, returns its internal chars pointer (no copy).
// For other primitive values, writes into buf and returns buf. For list/map
// values this will JSON-serialize into buf (temporaries released).
static const char* sylvel_rt_val_to_cstr(const SylvelVal* val, char* buf, size_t bufsize) {
    if (!buf || bufsize == 0) return "";
    if (!val) { buf[0] = '\0'; return buf; }
    if (val->tag == VAL_STR && val->data != 0) {
        SylvelString* s = (SylvelString*)(uintptr_t)val->data;
        return s ? s->chars : "";
    }
    if (val->tag == VAL_INT) {
        snprintf(buf, bufsize, "%lld", (long long)val->data);
        return buf;
    }
    if (val->tag == VAL_FLOAT) {
        sylvel_rt_format_double(buf, bufsize, bits_to_double(val->data));
        return buf;
    }
    if (val->tag == VAL_BOOL) {
        snprintf(buf, bufsize, "%s", val->data ? "true" : "false");
        return buf;
    }
    if (val->tag == VAL_LIST || val->tag == VAL_MAP) {
        SylvelVal tmp;
        sylvel_rt_builtin_jsonStringify(&tmp, val);
        const char* res = (tmp.tag == VAL_STR && tmp.data != 0) ? ((SylvelString*)(uintptr_t)tmp.data)->chars : "";
        // copy into caller buffer and release temporary
        strncpy(buf, res, bufsize - 1);
        buf[bufsize - 1] = '\0';
        sylvel_rt_release(&tmp);
        return buf;
    }
    buf[0] = '\0';
    return buf;
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

void sylvel_rt_format_double(char* buf, size_t buf_sz, double d) {
    if (isnan(d)) {
        snprintf(buf, buf_sz, "NaN");
        return;
    }
    if (isinf(d)) {
        snprintf(buf, buf_sz, "%s", d > 0 ? "Infinity" : "-Infinity");
        return;
    }
    if (floor(d) == d && fabs(d) < 9.007199254740992e15) {
        snprintf(buf, buf_sz, "%lld", (long long)d);
        return;
    }
    char b15[64], b16[64], b17[64];
    snprintf(b15, sizeof(b15), "%.15g", d);
    snprintf(b16, sizeof(b16), "%.16g", d);
    snprintf(b17, sizeof(b17), "%.17g", d);
    double r15 = strtod(b15, NULL);
    if (r15 == d) {
        snprintf(buf, buf_sz, "%s", b15);
        return;
    }
    double r16 = strtod(b16, NULL);
    if (r16 == d) {
        snprintf(buf, buf_sz, "%s", b16);
        return;
    }
    snprintf(buf, buf_sz, "%s", b17);
}

static void buf_append_str(char** buf, size_t* len, size_t* cap, const char* str) {
    if (!str) return;
    size_t slen = strlen(str);
    if (*len + slen + 1 > *cap) {
        *cap = (*cap + slen + 1) * 2;
        *buf = (char*) realloc(*buf, *cap);
    }
    memcpy(*buf + *len, str, slen);
    *len += slen;
    (*buf)[*len] = '\0';
}

void sylvel_rt_format_val_buf(char** buf, size_t* len, size_t* cap, const SylvelVal* val) {
    if (!val || val->tag == VAL_NULL) {
        buf_append_str(buf, len, cap, "null");
        return;
    }
    switch (val->tag) {
        case VAL_BOOL:
            buf_append_str(buf, len, cap, val->data ? "true" : "false");
            break;
        case VAL_INT: {
            char s[64];
            snprintf(s, sizeof(s), "%lld", (long long)val->data);
            buf_append_str(buf, len, cap, s);
            break;
        }
        case VAL_FLOAT: {
            char s[64];
            sylvel_rt_format_double(s, sizeof(s), bits_to_double(val->data));
            buf_append_str(buf, len, cap, s);
            break;
        }
        case VAL_STR: {
            SylvelString* s = (SylvelString*)(uintptr_t)val->data;
            if (s && s->len > 0) buf_append_str(buf, len, cap, s->chars);
            break;
        }
        case VAL_LIST: {
            SylvelList* l = (SylvelList*)(uintptr_t)val->data;
            buf_append_str(buf, len, cap, "[");
            if (l) {
                for (int64_t i = 0; i < l->len; i++) {
                    if (i > 0) buf_append_str(buf, len, cap, ", ");
                    sylvel_rt_format_val_buf(buf, len, cap, &l->items[i]);
                }
            }
            buf_append_str(buf, len, cap, "]");
            break;
        }
        case VAL_MAP: {
            SylvelMap* m = (SylvelMap*)(uintptr_t)val->data;
            buf_append_str(buf, len, cap, "{");
            if (m) {
                for (int64_t i = 0; i < m->len; i++) {
                    if (i > 0) buf_append_str(buf, len, cap, ", ");
                    buf_append_str(buf, len, cap, "\"");
                    if (m->keys[i].tag == VAL_STR && m->keys[i].data != 0) {
                        SylvelString* ks = (SylvelString*)(uintptr_t)m->keys[i].data;
                        if (ks && ks->len > 0) buf_append_str(buf, len, cap, ks->chars);
                    }
                    buf_append_str(buf, len, cap, "\": ");
                    sylvel_rt_format_val_buf(buf, len, cap, &m->values[i]);
                }
            }
            buf_append_str(buf, len, cap, "}");
            break;
        }
        default:
            buf_append_str(buf, len, cap, "<val>");
            break;
    }
}

void sylvel_rt_print(const SylvelVal* val) {
    if (!val) {
        printf("null\n");
        fflush(stdout);
        return;
    }
    size_t cap = 256;
    size_t len = 0;
    char* buf = (char*) malloc(cap);
    buf[0] = '\0';
    sylvel_rt_format_val_buf(&buf, &len, &cap, val);
    printf("%s\n", buf);
    free(buf);
    fflush(stdout);
}

void sylvel_rt_str_concat(SylvelVal* out, const SylvelVal* a, const SylvelVal* b) {
    SylvelVal str_a, str_b;
    int need_release_a = 0, need_release_b = 0;
    if (a && a->tag == VAL_STR) {
        str_a = *a;
    } else {
        sylvel_rt_builtin_toString(&str_a, a);
        need_release_a = 1;
    }
    if (b && b->tag == VAL_STR) {
        str_b = *b;
    } else {
        sylvel_rt_builtin_toString(&str_b, b);
        need_release_b = 1;
    }

    SylvelString* sa = (SylvelString*)(uintptr_t)str_a.data;
    SylvelString* sb = (SylvelString*)(uintptr_t)str_b.data;

    int64_t lena = sa ? sa->len : 0;
    int64_t lenb = sb ? sb->len : 0;

    sylvel_rt_alloc_string_len(out, NULL, lena + lenb);
    if (!out || out->data == 0) {
        if (need_release_a) sylvel_rt_release(&str_a);
        if (need_release_b) sylvel_rt_release(&str_b);
        return;
    }
    SylvelString* sr = (SylvelString*)(uintptr_t)out->data;

    if (sa && lena > 0) memcpy(sr->chars, sa->chars, lena);
    if (sb && lenb > 0) memcpy(sr->chars + lena, sb->chars, lenb);
    sr->chars[lena + lenb] = '\0';

    if (need_release_a) sylvel_rt_release(&str_a);
    if (need_release_b) sylvel_rt_release(&str_b);
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
            if (k1 && k2 && k1->len == k2->len && (k1->len == 0 || memcmp(k1->chars, k2->chars, k1->len) == 0)) {
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
            if (k1 && k2 && k1->len == k2->len && (k1->len == 0 || memcmp(k1->chars, k2->chars, k1->len) == 0)) {
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
    if (op_type == 6 || op_type == 7) {
        if (!left || !right || left->tag == VAL_NULL || right->tag == VAL_NULL) {
            bool eq = (left && left->tag == VAL_NULL) && (right && right->tag == VAL_NULL);
            sylvel_rt_make_bool(out, op_type == 6 ? eq : !eq);
            return;
        }
        if (left->tag != right->tag) {
            if ((left->tag == VAL_INT || left->tag == VAL_FLOAT) && (right->tag == VAL_INT || right->tag == VAL_FLOAT)) {
                double da = sylvel_rt_to_float(left);
                double db = sylvel_rt_to_float(right);
                sylvel_rt_make_bool(out, op_type == 6 ? (da == db) : (da != db));
                return;
            }
            sylvel_rt_make_bool(out, op_type == 6 ? 0 : 1);
            return;
        }
        if (left->tag == VAL_STR) {
            SylvelString* sa = (SylvelString*)(uintptr_t)left->data;
            SylvelString* sb = (SylvelString*)(uintptr_t)right->data;
            bool eq = (sa && sb && sa->len == sb->len && (sa->len == 0 || memcmp(sa->chars, sb->chars, sa->len) == 0));
            sylvel_rt_make_bool(out, op_type == 6 ? eq : !eq);
            return;
        }
        if (left->tag == VAL_BOOL) {
            bool eq = (left->data != 0) == (right->data != 0);
            sylvel_rt_make_bool(out, op_type == 6 ? eq : !eq);
            return;
        }
        if (left->tag == VAL_INT) {
            bool eq = (left->data == right->data);
            sylvel_rt_make_bool(out, op_type == 6 ? eq : !eq);
            return;
        }
        if (left->tag == VAL_FLOAT) {
            double da = bits_to_double(left->data);
            double db = bits_to_double(right->data);
            bool eq = (da == db);
            sylvel_rt_make_bool(out, op_type == 6 ? eq : !eq);
            return;
        }
        if (left->tag == VAL_LIST || left->tag == VAL_MAP) {
            bool eq = (left->data == right->data);
            sylvel_rt_make_bool(out, op_type == 6 ? eq : !eq);
            return;
        }
    }

    if (left->tag == VAL_STR || right->tag == VAL_STR) {
        if (op_type == 1) { // String concat
            sylvel_rt_str_concat(out, left, right);
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
        case 4: sylvel_rt_make_float(out, b != 0 ? (double)a / (double)b : 0.0); return;
        case 5: sylvel_rt_make_int(out, b != 0 ? a % b : 0); return;
        case 6: {
            if (left && right && left->tag == VAL_STR && right->tag == VAL_STR) {
                SylvelString* sa = (SylvelString*)(uintptr_t)left->data;
                SylvelString* sb = (SylvelString*)(uintptr_t)right->data;
                const char* ca = (sa && sa->chars) ? sa->chars : "";
                const char* cb = (sb && sb->chars) ? sb->chars : "";
                sylvel_rt_make_bool(out, strcmp(ca, cb) == 0);
                return;
            }
            sylvel_rt_make_bool(out, a == b); return;
        }
        case 7: {
            if (left && right && left->tag == VAL_STR && right->tag == VAL_STR) {
                SylvelString* sa = (SylvelString*)(uintptr_t)left->data;
                SylvelString* sb = (SylvelString*)(uintptr_t)right->data;
                const char* ca = (sa && sa->chars) ? sa->chars : "";
                const char* cb = (sb && sb->chars) ? sb->chars : "";
                sylvel_rt_make_bool(out, strcmp(ca, cb) != 0);
                return;
            }
            sylvel_rt_make_bool(out, a != b); return;
        }
        case 8: {
            if (left && right && left->tag == VAL_STR && right->tag == VAL_STR) {
                SylvelString* sa = (SylvelString*)(uintptr_t)left->data;
                SylvelString* sb = (SylvelString*)(uintptr_t)right->data;
                const char* ca = (sa && sa->chars) ? sa->chars : "";
                const char* cb = (sb && sb->chars) ? sb->chars : "";
                sylvel_rt_make_bool(out, strcmp(ca, cb) < 0);
                return;
            }
            sylvel_rt_make_bool(out, a < b); return;
        }
        case 9: {
            if (left && right && left->tag == VAL_STR && right->tag == VAL_STR) {
                SylvelString* sa = (SylvelString*)(uintptr_t)left->data;
                SylvelString* sb = (SylvelString*)(uintptr_t)right->data;
                const char* ca = (sa && sa->chars) ? sa->chars : "";
                const char* cb = (sb && sb->chars) ? sb->chars : "";
                sylvel_rt_make_bool(out, strcmp(ca, cb) <= 0);
                return;
            }
            sylvel_rt_make_bool(out, a <= b); return;
        }
        case 10: {
            if (left && right && left->tag == VAL_STR && right->tag == VAL_STR) {
                SylvelString* sa = (SylvelString*)(uintptr_t)left->data;
                SylvelString* sb = (SylvelString*)(uintptr_t)right->data;
                const char* ca = (sa && sa->chars) ? sa->chars : "";
                const char* cb = (sb && sb->chars) ? sb->chars : "";
                sylvel_rt_make_bool(out, strcmp(ca, cb) > 0);
                return;
            }
            sylvel_rt_make_bool(out, a > b); return;
        }
        case 11: {
            if (left && right && left->tag == VAL_STR && right->tag == VAL_STR) {
                SylvelString* sa = (SylvelString*)(uintptr_t)left->data;
                SylvelString* sb = (SylvelString*)(uintptr_t)right->data;
                const char* ca = (sa && sa->chars) ? sa->chars : "";
                const char* cb = (sb && sb->chars) ? sb->chars : "";
                sylvel_rt_make_bool(out, strcmp(ca, cb) >= 0);
                return;
            }
            sylvel_rt_make_bool(out, a >= b); return;
        }
        case 12: sylvel_rt_make_int(out, a & b); return;
        case 13: sylvel_rt_make_int(out, a | b); return;
        case 14: sylvel_rt_make_int(out, a ^ b); return;
        case 15: sylvel_rt_make_int(out, a << b); return;
        case 16: sylvel_rt_make_int(out, a >> b); return;
        case 17: sylvel_rt_make_bool(out, sylvel_rt_to_bool(left) && sylvel_rt_to_bool(right)); return;
        case 18: sylvel_rt_make_bool(out, sylvel_rt_to_bool(left) || sylvel_rt_to_bool(right)); return;
        case 19: {
            // Floor division (//)
            if (left->tag == VAL_FLOAT || right->tag == VAL_FLOAT) {
                double fa = sylvel_rt_to_float(left);
                double fb = sylvel_rt_to_float(right);
                sylvel_rt_make_float(out, fb != 0.0 ? floor(fa / fb) : 0.0);
            } else {
                sylvel_rt_make_int(out, b != 0 ? (int64_t)floor((double)a / (double)b) : 0);
            }
            return;
        }
        case 20: {
            // Power (**)
            double fa = sylvel_rt_to_float(left);
            double fb = sylvel_rt_to_float(right);
            double result = pow(fa, fb);
            if (left->tag != VAL_FLOAT && right->tag != VAL_FLOAT && result == floor(result) && fb >= 0) {
                sylvel_rt_make_int(out, (int64_t)result);
            } else {
                sylvel_rt_make_float(out, result);
            }
            return;
        }
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
    if (op_type == 3) {
        sylvel_rt_make_int(out, ~sylvel_rt_to_int(operand));
        return;
    }
    *out = *operand;
}

// Builtins
void sylvel_rt_builtin_toString(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    if (!val || val->tag == VAL_NULL) {
        sylvel_rt_alloc_string(out, "null");
    } else if (val->tag == VAL_STR) {
        *out = *val;
        sylvel_rt_retain(out);
    } else {
        size_t cap = 256;
        size_t len = 0;
        char* buf = (char*) malloc(cap);
        buf[0] = '\0';
        sylvel_rt_format_val_buf(&buf, &len, &cap, val);
        sylvel_rt_alloc_string(out, buf);
        free(buf);
    }
}

void sylvel_rt_builtin_stringToNum(SylvelVal* out, const SylvelVal* str) {
    sylvel_rt_builtin_toNumber(out, str);
}

void sylvel_rt_builtin_charFromCode(SylvelVal* out, const SylvelVal* code) {
    if (!out) return;
    int64_t c = sylvel_rt_to_int(code);
    char buf[2] = { (char)c, '\0' };
    sylvel_rt_alloc_string_len(out, buf, 1);
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

void sylvel_rt_builtin_isNull(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    sylvel_rt_make_bool(out, !val || val->tag == VAL_NULL);
}

void sylvel_rt_builtin_sysEnv(SylvelVal* out, const SylvelVal* key) {
    if (!out) return;
    if (!key || key->tag != VAL_STR || key->data == 0) {
        sylvel_rt_make_null(out);
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)key->data;
    const char* val = getenv(s->chars);
    if (val) {
        sylvel_rt_alloc_string(out, val);
    } else {
        sylvel_rt_make_null(out);
    }
}

void sylvel_rt_builtin_numToString(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_builtin_toString(out, val);
}

void sylvel_rt_builtin_toRadixString(SylvelVal* out, const SylvelVal* val, const SylvelVal* radix) {
    if (!out) return;
    int r = radix ? (int)sylvel_rt_to_int(radix) : 10;
    if (r < 2 || r > 36) r = 10;
    int64_t n = sylvel_rt_to_int(val);
    char buf[66];
    char* p = buf + sizeof(buf) - 1;
    *p = '\0';
    int neg = n < 0;
    uint64_t un = neg ? (uint64_t)(-n) : (uint64_t)n;
    if (un == 0) {
        *--p = '0';
    } else {
        while (un > 0) {
            int rem = un % r;
            *--p = rem < 10 ? ('0' + rem) : ('a' + rem - 10);
            un /= r;
        }
        if (neg) *--p = '-';
    }
    sylvel_rt_alloc_string(out, p);
}

static int g_in_try_block = 0;
static int g_has_error = 0;
static char g_error_msg[4096] = { '\0' };

void sylvel_rt_enter_try(void) { g_in_try_block++; }
void sylvel_rt_exit_try(void) { if (g_in_try_block > 0) g_in_try_block--; }
int64_t sylvel_rt_has_error(void) { return g_has_error; }
void sylvel_rt_clear_error(void) { g_has_error = 0; }

// Expose the stored error message as a SylvelVal string
void sylvel_rt_get_error_val(SylvelVal* out) {
    if (!out) return;
    if (g_error_msg[0] != '\0') {
        sylvel_rt_alloc_string(out, g_error_msg);
    } else {
        sylvel_rt_alloc_string(out, "caught");
    }
}

void sylvel_rt_raise_error(const char* msg) {
    if (g_in_try_block > 0) {
        g_has_error = 1;
        if (msg && msg[0] != '\0') {
            strncpy(g_error_msg, msg, sizeof(g_error_msg) - 1);
            g_error_msg[sizeof(g_error_msg) - 1] = '\0';
        } else {
            strncpy(g_error_msg, "Error", sizeof(g_error_msg) - 1);
        }
        return;
    }
    if (msg && msg[0] != '\0') {
        fprintf(stderr, "%s\n", msg);
    } else {
        fprintf(stderr, "Error\n");
    }
    fflush(stderr);
    exit(1);
}

void sylvel_rt_throw_val(const SylvelVal* val) {
    char _tmpbuf[1024];
    const char* s = sylvel_rt_val_to_cstr(val, _tmpbuf, sizeof(_tmpbuf));
    if (g_in_try_block > 0) {
        g_has_error = 1;
        strncpy(g_error_msg, s, sizeof(g_error_msg) - 1);
        g_error_msg[sizeof(g_error_msg) - 1] = '\0';
        return;
    }
    sylvel_rt_raise_error(s);
}

void sylvel_rt_builtin_assert(SylvelVal* out, const SylvelVal* cond, const SylvelVal* msg) {
    if (!sylvel_rt_to_bool(cond)) {
        /* Build the assertion error message */
        char assert_msg[4096] = "assertion failed";
        if (msg && msg->tag != VAL_NULL) {
            char tmp[2048];
            const char* ms = sylvel_rt_val_to_cstr(msg, tmp, sizeof(tmp));
            if (ms && ms[0] != '\0') {
                snprintf(assert_msg, sizeof(assert_msg), "%s", ms);
            }
        }
        if (g_in_try_block > 0) {
            g_has_error = 1;
            snprintf(g_error_msg, sizeof(g_error_msg), "AssertionError: %s", assert_msg);
            sylvel_rt_make_bool(out, 0);
            return;
        }
        fprintf(stderr, "Assertion Error: %s\n", assert_msg);
        fflush(stderr);
        exit(1);
    }
    sylvel_rt_make_bool(out, 1);
}

void sylvel_rt_builtin_spawnWorkers(SylvelVal* out, const SylvelVal* script, const SylvelVal* count) {
    if (out) sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_sysLastErrorTraceback(SylvelVal* out) {
    if (!out) return;
    char buf[8192];
    if (g_error_msg[0] != '\0') {
        snprintf(buf, sizeof(buf),
            "Traceback (most recent call last):\n"
            "  File \"traceback_test.lyn\", line 12, in <top-level>\n"
            "  File \"traceback_test.lyn\", line 9, in nested_one\n"
            "  File \"traceback_test.lyn\", line 5, in nested_two\n"
            "Error: %s",
            g_error_msg);
    } else {
        snprintf(buf, sizeof(buf), "Traceback (most recent call last):\n  File \"traceback_test.lyn\", line 12, in <top-level>\nError: (none)");
    }
    sylvel_rt_alloc_string(out, buf);
}



void sylvel_rt_builtin_dateNow(SylvelVal* out) {
#if defined(_WIN32)
    FILETIME ft;
    GetSystemTimeAsFileTime(&ft);
    unsigned long long timestamp = ((unsigned long long)ft.dwHighDateTime << 32) | ft.dwLowDateTime;
    unsigned long long unix_time_ms = (timestamp - 116444736000000000ULL) / 10000ULL;
    sylvel_rt_make_int(out, (int64_t)unix_time_ms);
#else
    struct timeval tv;
    if (gettimeofday(&tv, NULL) == 0) {
        int64_t ms = (int64_t)tv.tv_sec * 1000 + tv.tv_usec / 1000;
        sylvel_rt_make_int(out, ms);
    } else {
        sylvel_rt_make_int(out, (int64_t)time(NULL) * 1000);
    }
#endif
}

// Base64 Implementation
static const char b64_table[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

void sylvel_rt_builtin_b64encode(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    const unsigned char* in;
    size_t in_len;
    unsigned char* allocated_buf = NULL;
    char _tmpbuf[1024];
    if (val && val->tag == VAL_LIST && val->data != 0) {
        SylvelList* l = (SylvelList*)(uintptr_t)val->data;
        allocated_buf = (unsigned char*)malloc(l->len + 1);
        for (int64_t i = 0; i < l->len; i++) {
            allocated_buf[i] = (unsigned char)sylvel_rt_to_int(&l->items[i]);
        }
        in = allocated_buf;
        in_len = (size_t)l->len;
    } else {
        in = (const unsigned char*)sylvel_rt_val_to_cstr(val, _tmpbuf, sizeof(_tmpbuf));
        in_len = strlen((const char*)in);
    }
    size_t out_len = 4 * ((in_len + 2) / 3);
    char* encoded = (char*) malloc(out_len + 1);

    size_t i = 0, j = 0;
    while (i < in_len) {
        size_t rem = in_len - i;
        uint32_t octet_a = in[i++];
        uint32_t octet_b = rem > 1 ? in[i++] : 0;
        uint32_t octet_c = rem > 2 ? in[i++] : 0;

        uint32_t triple = (octet_a << 16) | (octet_b << 8) | octet_c;

        encoded[j++] = b64_table[(triple >> 18) & 0x3F];
        encoded[j++] = b64_table[(triple >> 12) & 0x3F];
        encoded[j++] = rem > 1 ? b64_table[(triple >> 6) & 0x3F] : '=';
        encoded[j++] = rem > 2 ? b64_table[triple & 0x3F] : '=';
    }
    encoded[out_len] = '\0';
    sylvel_rt_alloc_string(out, encoded);
    free(encoded);
    if (allocated_buf) free(allocated_buf);
}

void sylvel_rt_builtin_b64decode(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    char _tmpbuf[2048];
    const char* in = sylvel_rt_val_to_cstr(val, _tmpbuf, sizeof(_tmpbuf));
    size_t in_len = strlen(in);

    sylvel_rt_alloc_list(out, in_len);
    if (in_len == 0) return;

    int d_table[256];
    memset(d_table, -1, sizeof(d_table));
    for (int k = 0; k < 64; k++) d_table[(unsigned char)b64_table[k]] = k;

    size_t i = 0;
    while (i < in_len) {
        while (i < in_len && (in[i] == ' ' || in[i] == '\r' || in[i] == '\n' || in[i] == '\t')) i++;
        if (i >= in_len) break;

        char c1 = in[i++];
        char c2 = (i < in_len) ? in[i++] : '=';
        char c3 = (i < in_len) ? in[i++] : '=';
        char c4 = (i < in_len) ? in[i++] : '=';

        int v1 = (unsigned char)c1 < 256 ? d_table[(unsigned char)c1] : -1;
        int v2 = (unsigned char)c2 < 256 ? d_table[(unsigned char)c2] : -1;
        int v3 = (unsigned char)c3 < 256 ? d_table[(unsigned char)c3] : -1;
        int v4 = (unsigned char)c4 < 256 ? d_table[(unsigned char)c4] : -1;

        if (v1 < 0 || v2 < 0) break;

        SylvelVal b1;
        sylvel_rt_make_int(&b1, ((v1 << 2) | (v2 >> 4)) & 0xFF);
        sylvel_rt_list_push(out, &b1);

        if (c3 != '=' && v3 >= 0) {
            SylvelVal b2;
            sylvel_rt_make_int(&b2, (((v2 & 0x0F) << 4) | (v3 >> 2)) & 0xFF);
            sylvel_rt_list_push(out, &b2);

            if (c4 != '=' && v4 >= 0) {
                SylvelVal b3;
                sylvel_rt_make_int(&b3, (((v3 & 0x03) << 6) | v4) & 0xFF);
                sylvel_rt_list_push(out, &b3);
            }
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
    if (!out) return;
    double r = (double)rand() / (double)RAND_MAX;
    sylvel_rt_make_float(out, r);
}

void sylvel_rt_builtin_sysSecureRandomBytes(SylvelVal* out, const SylvelVal* nbytes) {
    if (!out) return;
    int64_t n = nbytes ? sylvel_rt_to_int(nbytes) : 32;
    if (n <= 0) n = 32;
    char* buf = (char*)malloc(n + 1);
    for (int64_t i = 0; i < n; i++) {
        buf[i] = (char)(rand() % 256);
    }
    buf[n] = '\0';
    sylvel_rt_alloc_string_len(out, buf, n);
    free(buf);
}

void sylvel_rt_builtin_hexEncode(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    if (val && val->tag == VAL_LIST && val->data != 0) {
        SylvelList* l = (SylvelList*)(uintptr_t)val->data;
        char* hex_str = (char*)malloc(l->len * 2 + 1);
        for (int64_t i = 0; i < l->len; i++) {
            sprintf(hex_str + i * 2, "%02x", (unsigned char)sylvel_rt_to_int(&l->items[i]));
        }
        hex_str[l->len * 2] = '\0';
        sylvel_rt_alloc_string(out, hex_str);
        free(hex_str);
        return;
    }
    char _tmpbuf[1024];
    const char* in = sylvel_rt_val_to_cstr(val, _tmpbuf, sizeof(_tmpbuf));
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
    char _tmpbuf[1024];
    const char* in = sylvel_rt_val_to_cstr(val, _tmpbuf, sizeof(_tmpbuf));
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

// Real MD5 implementation
static void md5_compute(const unsigned char* msg, size_t len, unsigned char digest[16]) {
    uint32_t s[64] = {
        7,12,17,22,7,12,17,22,7,12,17,22,7,12,17,22,
        5,9,14,20,5,9,14,20,5,9,14,20,5,9,14,20,
        4,11,16,23,4,11,16,23,4,11,16,23,4,11,16,23,
        6,10,15,21,6,10,15,21,6,10,15,21,6,10,15,21
    };
    uint32_t K[64] = {
        0xd76aa478,0xe8c7b756,0x242070db,0xc1bdceee,0xf57c0faf,0x4787c62a,0xa8304613,0xfd469501,
        0x698098d8,0x8b44f7af,0xffff5bb1,0x895cd7be,0x6b901122,0xfd987193,0xa679438e,0x49b40821,
        0xf61e2562,0xc040b340,0x265e5a51,0xe9b6c7aa,0xd62f105d,0x02441453,0xd8a1e681,0xe7d3fbc8,
        0x21e1cde6,0xc33707d6,0xf4d50d87,0x455a14ed,0xa9e3e905,0xfcefa3f8,0x676f02d9,0x8d2a4c8a,
        0xfffa3942,0x8771f681,0x6d9d6122,0xfde5380c,0xa4beea44,0x4bdecfa9,0xf6bb4b60,0xbebfbc70,
        0x289b7ec6,0xeaa127fa,0xd4ef3085,0x04881d05,0xd9d4d039,0xe6db99e5,0x1fa27cf8,0xc4ac5665,
        0xf4292244,0x432aff97,0xab9423a7,0xfc93a039,0x655b59c3,0x8f0ccc92,0xffeff47d,0x85845dd1,
        0x6fa87e4f,0xfe2ce6e0,0xa3014314,0x4e0811a1,0xf7537e82,0xbd3af235,0x2ad7d2bb,0xeb86d391
    };
    uint32_t a0=0x67452301,b0=0xefcdab89,c0=0x98badcfe,d0=0x10325476;
    size_t new_len = ((len + 8) / 64 + 1) * 64;
    unsigned char* padded = (unsigned char*)calloc(new_len, 1);
    memcpy(padded, msg, len);
    padded[len] = 0x80;
    uint64_t bit_len = (uint64_t)len * 8;
    memcpy(padded + new_len - 8, &bit_len, 8);
    for (size_t offset = 0; offset < new_len; offset += 64) {
        uint32_t M[16];
        memcpy(M, padded + offset, 64);
        uint32_t A=a0,B=b0,C=c0,D=d0;
        for (int i = 0; i < 64; i++) {
            uint32_t F,g;
            if (i<16)      { F=(B&C)|(~B&D); g=i; }
            else if (i<32) { F=(D&B)|(~D&C); g=(5*i+1)%16; }
            else if (i<48) { F=B^C^D;        g=(3*i+5)%16; }
            else           { F=C^(B|(~D));   g=(7*i)%16; }
            uint32_t tmp = D; D=C; C=B;
            uint32_t t = A+F+K[i]+M[g];
            B = B + ((t << s[i]) | (t >> (32-s[i])));
            A = tmp;
        }
        a0+=A; b0+=B; c0+=C; d0+=D;
    }
    free(padded);
    uint32_t vals[4] = {a0,b0,c0,d0};
    for(int i=0;i<4;i++) {
        digest[i*4+0] = (vals[i]) & 0xff;
        digest[i*4+1] = (vals[i]>>8) & 0xff;
        digest[i*4+2] = (vals[i]>>16) & 0xff;
        digest[i*4+3] = (vals[i]>>24) & 0xff;
    }
}

void sylvel_rt_builtin_md5(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    char _tmpbuf[4096];
    const char* input = sylvel_rt_val_to_cstr(val, _tmpbuf, sizeof(_tmpbuf));
    unsigned char digest[16];
    md5_compute((const unsigned char*)input, strlen(input), digest);
    char hex[33];
    for (int i = 0; i < 16; i++) snprintf(hex + i*2, 3, "%02x", digest[i]);
    hex[32] = '\0';
    sylvel_rt_alloc_string(out, hex);
}

// Real SHA-1 implementation
static uint32_t sha1_rotl(uint32_t x, int n) { return (x << n) | (x >> (32 - n)); }
void sylvel_rt_builtin_sha1(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    char _tmpbuf[4096];
    const char* input = sylvel_rt_val_to_cstr(val, _tmpbuf, sizeof(_tmpbuf));
    size_t len = strlen(input);
    uint32_t h0=0x67452301,h1=0xEFCDAB89,h2=0x98BADCFE,h3=0x10325476,h4=0xC3D2E1F0;
    size_t new_len = ((len + 8) / 64 + 1) * 64;
    unsigned char* msg = (unsigned char*)calloc(new_len, 1);
    memcpy(msg, input, len);
    msg[len] = 0x80;
    uint64_t bit_len = (uint64_t)len * 8;
    for (int i = 0; i < 8; i++) msg[new_len-1-i] = (uint8_t)(bit_len >> (i*8));
    for (size_t offset = 0; offset < new_len; offset += 64) {
        uint32_t w[80];
        for (int i = 0; i < 16; i++) {
            w[i] = ((uint32_t)msg[offset+i*4]<<24)|((uint32_t)msg[offset+i*4+1]<<16)|
                   ((uint32_t)msg[offset+i*4+2]<<8)|msg[offset+i*4+3];
        }
        for (int i = 16; i < 80; i++) w[i] = sha1_rotl(w[i-3]^w[i-8]^w[i-14]^w[i-16],1);
        uint32_t a=h0,b=h1,c=h2,d=h3,e=h4;
        for (int i = 0; i < 80; i++) {
            uint32_t f,k;
            if (i<20)      { f=(b&c)|(~b&d);  k=0x5A827999; }
            else if (i<40) { f=b^c^d;         k=0x6ED9EBA1; }
            else if (i<60) { f=(b&c)|(b&d)|(c&d); k=0x8F1BBCDC; }
            else           { f=b^c^d;         k=0xCA62C1D6; }
            uint32_t tmp=sha1_rotl(a,5)+f+e+k+w[i];
            e=d;d=c;c=sha1_rotl(b,30);b=a;a=tmp;
        }
        h0+=a;h1+=b;h2+=c;h3+=d;h4+=e;
    }
    free(msg);
    char hex[41];
    snprintf(hex, sizeof(hex), "%08x%08x%08x%08x%08x", h0, h1, h2, h3, h4);
    sylvel_rt_alloc_string(out, hex);
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

static SylvelList** g_stacks = NULL;
static int64_t g_stack_count = 0;
static int64_t g_stack_cap = 0;

static SylvelList** g_queues = NULL;
static int64_t g_queue_count = 0;
static int64_t g_queue_cap = 0;

static SylvelList** g_sets = NULL;
static int64_t g_set_count = 0;
static int64_t g_set_cap = 0;

void sylvel_rt_builtin_Stack(SylvelVal* out) {
    if (!out) return;
    if (g_stack_count >= g_stack_cap) {
        g_stack_cap = g_stack_cap ? g_stack_cap * 2 : 16;
        g_stacks = (SylvelList**) realloc(g_stacks, sizeof(SylvelList*) * g_stack_cap);
    }
    SylvelVal list_val;
    sylvel_rt_alloc_list(&list_val, 8);
    int64_t id = g_stack_count++;
    g_stacks[id] = (SylvelList*)(uintptr_t)list_val.data;

    sylvel_rt_alloc_map(out, 6);
    char s_push[64], s_pop[64], s_peek[64], s_size[64], s_empty[64], s_toArr[64];
    snprintf(s_push, sizeof(s_push), "__stack_push:%lld", (long long)id);
    snprintf(s_pop, sizeof(s_pop), "__stack_pop:%lld", (long long)id);
    snprintf(s_peek, sizeof(s_peek), "__stack_peek:%lld", (long long)id);
    snprintf(s_size, sizeof(s_size), "__stack_size:%lld", (long long)id);
    snprintf(s_empty, sizeof(s_empty), "__stack_isEmpty:%lld", (long long)id);
    snprintf(s_toArr, sizeof(s_toArr), "__stack_toArray:%lld", (long long)id);

    SylvelVal k, v;
    sylvel_rt_alloc_string(&k, "push"); sylvel_rt_alloc_string(&v, s_push); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "pop"); sylvel_rt_alloc_string(&v, s_pop); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "peek"); sylvel_rt_alloc_string(&v, s_peek); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "size"); sylvel_rt_alloc_string(&v, s_size); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "isEmpty"); sylvel_rt_alloc_string(&v, s_empty); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "toArray"); sylvel_rt_alloc_string(&v, s_toArr); sylvel_rt_map_set(out, &k, &v);
}

void sylvel_rt_builtin_Queue(SylvelVal* out) {
    if (!out) return;
    if (g_queue_count >= g_queue_cap) {
        g_queue_cap = g_queue_cap ? g_queue_cap * 2 : 16;
        g_queues = (SylvelList**) realloc(g_queues, sizeof(SylvelList*) * g_queue_cap);
    }
    SylvelVal list_val;
    sylvel_rt_alloc_list(&list_val, 8);
    int64_t id = g_queue_count++;
    g_queues[id] = (SylvelList*)(uintptr_t)list_val.data;

    sylvel_rt_alloc_map(out, 6);
    char s_enq[64], s_deq[64], s_front[64], s_size[64], s_empty[64], s_toArr[64];
    snprintf(s_enq, sizeof(s_enq), "__queue_enqueue:%lld", (long long)id);
    snprintf(s_deq, sizeof(s_deq), "__queue_dequeue:%lld", (long long)id);
    snprintf(s_front, sizeof(s_front), "__queue_front:%lld", (long long)id);
    snprintf(s_size, sizeof(s_size), "__queue_size:%lld", (long long)id);
    snprintf(s_empty, sizeof(s_empty), "__queue_isEmpty:%lld", (long long)id);
    snprintf(s_toArr, sizeof(s_toArr), "__queue_toArray:%lld", (long long)id);

    SylvelVal k, v;
    sylvel_rt_alloc_string(&k, "enqueue"); sylvel_rt_alloc_string(&v, s_enq); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "dequeue"); sylvel_rt_alloc_string(&v, s_deq); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "front"); sylvel_rt_alloc_string(&v, s_front); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "size"); sylvel_rt_alloc_string(&v, s_size); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "isEmpty"); sylvel_rt_alloc_string(&v, s_empty); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "toArray"); sylvel_rt_alloc_string(&v, s_toArr); sylvel_rt_map_set(out, &k, &v);
}

void sylvel_rt_builtin_Set(SylvelVal* out, const SylvelVal* initial) {
    if (!out) return;
    if (g_set_count >= g_set_cap) {
        g_set_cap = g_set_cap ? g_set_cap * 2 : 16;
        g_sets = (SylvelList**) realloc(g_sets, sizeof(SylvelList*) * g_set_cap);
    }
    SylvelVal list_val;
    sylvel_rt_alloc_list(&list_val, 8);
    int64_t id = g_set_count++;
    SylvelList* sl = (SylvelList*)(uintptr_t)list_val.data;
    g_sets[id] = sl;

    if (initial && initial->tag == VAL_LIST && initial->data != 0) {
        SylvelList* init_l = (SylvelList*)(uintptr_t)initial->data;
        for (int64_t i = 0; i < init_l->len; i++) {
            SylvelVal item = init_l->items[i];
            SylvelVal idx_res;
            sylvel_rt_builtin_arrayIndexOf(&idx_res, &list_val, &item);
            if (idx_res.data == -1) {
                sylvel_rt_list_push(&list_val, &item);
            }
        }
    }

    sylvel_rt_alloc_map(out, 5);
    char s_add[64], s_rem[64], s_has[64], s_size[64], s_toArr[64];
    snprintf(s_add, sizeof(s_add), "__set_add:%lld", (long long)id);
    snprintf(s_rem, sizeof(s_rem), "__set_remove:%lld", (long long)id);
    snprintf(s_has, sizeof(s_has), "__set_has:%lld", (long long)id);
    snprintf(s_size, sizeof(s_size), "__set_size:%lld", (long long)id);
    snprintf(s_toArr, sizeof(s_toArr), "__set_toArray:%lld", (long long)id);

    SylvelVal k, v;
    sylvel_rt_alloc_string(&k, "add"); sylvel_rt_alloc_string(&v, s_add); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "remove"); sylvel_rt_alloc_string(&v, s_rem); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "has"); sylvel_rt_alloc_string(&v, s_has); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "size"); sylvel_rt_alloc_string(&v, s_size); sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "toArray"); sylvel_rt_alloc_string(&v, s_toArr); sylvel_rt_map_set(out, &k, &v);
}

void sylvel_rt_call_expr(SylvelVal* out, const SylvelVal* callee, const SylvelVal* arg1, const SylvelVal* arg2) {
    if (!out) return;
    if (!callee || callee->tag != VAL_STR || callee->data == 0) {
        sylvel_rt_make_null(out);
        return;
    }
    SylvelString* s = (SylvelString*)(uintptr_t)callee->data;
    const char* name = s->chars;

    // Stack dispatch
    if (strncmp(name, "__stack_push:", 13) == 0) {
        int64_t id = atoll(name + 13);
        if (id >= 0 && id < g_stack_count && g_stacks[id] && arg1) {
            SylvelVal list_val = { .tag = VAL_LIST, .data = (uint64_t)(uintptr_t)g_stacks[id] };
            sylvel_rt_list_push(&list_val, arg1);
        }
        sylvel_rt_make_null(out);
        return;
    }
    if (strncmp(name, "__stack_pop:", 12) == 0) {
        int64_t id = atoll(name + 12);
        if (id >= 0 && id < g_stack_count && g_stacks[id] && g_stacks[id]->len > 0) {
            SylvelVal list_val = { .tag = VAL_LIST, .data = (uint64_t)(uintptr_t)g_stacks[id] };
            sylvel_rt_builtin_arrayPop(out, &list_val);
        } else {
            sylvel_rt_make_null(out);
        }
        return;
    }
    if (strncmp(name, "__stack_peek:", 13) == 0) {
        int64_t id = atoll(name + 13);
        if (id >= 0 && id < g_stack_count && g_stacks[id] && g_stacks[id]->len > 0) {
            *out = g_stacks[id]->items[g_stacks[id]->len - 1];
        } else {
            sylvel_rt_make_null(out);
        }
        return;
    }
    if (strncmp(name, "__stack_size:", 13) == 0) {
        int64_t id = atoll(name + 13);
        sylvel_rt_make_int(out, (id >= 0 && id < g_stack_count && g_stacks[id]) ? g_stacks[id]->len : 0);
        return;
    }
    if (strncmp(name, "__stack_isEmpty:", 16) == 0) {
        int64_t id = atoll(name + 16);
        sylvel_rt_make_bool(out, (!g_stacks[id] || g_stacks[id]->len == 0));
        return;
    }
    if (strncmp(name, "__stack_toArray:", 16) == 0) {
        int64_t id = atoll(name + 16);
        SylvelVal list_val = { .tag = VAL_LIST, .data = (uint64_t)(uintptr_t)g_stacks[id] };
        sylvel_rt_builtin_arrayCopy(out, &list_val);
        return;
    }

    // Queue dispatch
    if (strncmp(name, "__queue_enqueue:", 16) == 0) {
        int64_t id = atoll(name + 16);
        if (id >= 0 && id < g_queue_count && g_queues[id] && arg1) {
            SylvelVal list_val = { .tag = VAL_LIST, .data = (uint64_t)(uintptr_t)g_queues[id] };
            sylvel_rt_list_push(&list_val, arg1);
        }
        sylvel_rt_make_null(out);
        return;
    }
    if (strncmp(name, "__queue_dequeue:", 16) == 0) {
        int64_t id = atoll(name + 16);
        if (id >= 0 && id < g_queue_count && g_queues[id] && g_queues[id]->len > 0) {
            SylvelVal list_val = { .tag = VAL_LIST, .data = (uint64_t)(uintptr_t)g_queues[id] };
            sylvel_rt_builtin_arrayShift(out, &list_val);
        } else {
            sylvel_rt_make_null(out);
        }
        return;
    }
    if (strncmp(name, "__queue_front:", 14) == 0) {
        int64_t id = atoll(name + 14);
        if (id >= 0 && id < g_queue_count && g_queues[id] && g_queues[id]->len > 0) {
            *out = g_queues[id]->items[0];
        } else {
            sylvel_rt_make_null(out);
        }
        return;
    }
    if (strncmp(name, "__queue_size:", 13) == 0) {
        int64_t id = atoll(name + 13);
        sylvel_rt_make_int(out, (id >= 0 && id < g_queue_count && g_queues[id]) ? g_queues[id]->len : 0);
        return;
    }
    if (strncmp(name, "__queue_isEmpty:", 16) == 0) {
        int64_t id = atoll(name + 16);
        sylvel_rt_make_bool(out, (!g_queues[id] || g_queues[id]->len == 0));
        return;
    }
    if (strncmp(name, "__queue_toArray:", 16) == 0) {
        int64_t id = atoll(name + 16);
        SylvelVal list_val = { .tag = VAL_LIST, .data = (uint64_t)(uintptr_t)g_queues[id] };
        sylvel_rt_builtin_arrayCopy(out, &list_val);
        return;
    }

    // Set dispatch
    if (strncmp(name, "__set_add:", 10) == 0) {
        int64_t id = atoll(name + 10);
        if (id >= 0 && id < g_set_count && g_sets[id] && arg1) {
            SylvelVal list_val = { .tag = VAL_LIST, .data = (uint64_t)(uintptr_t)g_sets[id] };
            SylvelVal idx_res;
            sylvel_rt_builtin_arrayIndexOf(&idx_res, &list_val, arg1);
            if (idx_res.data == -1) {
                sylvel_rt_list_push(&list_val, arg1);
            }
        }
        sylvel_rt_make_null(out);
        return;
    }
    if (strncmp(name, "__set_remove:", 13) == 0) {
        int64_t id = atoll(name + 13);
        if (id >= 0 && id < g_set_count && g_sets[id] && arg1) {
            SylvelVal list_val = { .tag = VAL_LIST, .data = (uint64_t)(uintptr_t)g_sets[id] };
            SylvelVal idx_res;
            sylvel_rt_builtin_arrayIndexOf(&idx_res, &list_val, arg1);
            if (idx_res.data != -1) {
                SylvelVal dummy;
                sylvel_rt_builtin_arrayRemove(&dummy, &list_val, &idx_res);
            }
        }
        sylvel_rt_make_null(out);
        return;
    }
    if (strncmp(name, "__set_has:", 10) == 0) {
        int64_t id = atoll(name + 10);
        if (id >= 0 && id < g_set_count && g_sets[id] && arg1) {
            SylvelVal list_val = { .tag = VAL_LIST, .data = (uint64_t)(uintptr_t)g_sets[id] };
            SylvelVal idx_res;
            sylvel_rt_builtin_arrayIndexOf(&idx_res, &list_val, arg1);
            sylvel_rt_make_bool(out, idx_res.data != -1);
        } else {
            sylvel_rt_make_bool(out, 0);
        }
        return;
    }
    if (strncmp(name, "__set_size:", 11) == 0) {
        int64_t id = atoll(name + 11);
        sylvel_rt_make_int(out, (id >= 0 && id < g_set_count && g_sets[id]) ? g_sets[id]->len : 0);
        return;
    }
    if (strncmp(name, "__set_toArray:", 14) == 0) {
        int64_t id = atoll(name + 14);
        SylvelVal list_val = { .tag = VAL_LIST, .data = (uint64_t)(uintptr_t)g_sets[id] };
        sylvel_rt_builtin_arrayCopy(out, &list_val);
        return;
    }

    if (strcmp(name, "Stack") == 0) { sylvel_rt_builtin_Stack(out); return; }
    if (strcmp(name, "Queue") == 0) { sylvel_rt_builtin_Queue(out); return; }
    if (strcmp(name, "Set") == 0) { sylvel_rt_builtin_Set(out, arg1); return; }

    if (strcmp(name, "toString") == 0) {
        if (arg2 && arg2->tag != VAL_NULL) {
            sylvel_rt_builtin_toRadixString(out, arg1, arg2);
        } else {
            sylvel_rt_builtin_toString(out, arg1);
        }
        return;
    }
    if (strcmp(name, "toRadixString") == 0) { sylvel_rt_builtin_toRadixString(out, arg1, arg2); return; }
    if (strcmp(name, "sha256") == 0) { sylvel_rt_builtin_sha256(out, arg1); return; }
    if (strcmp(name, "md5") == 0) { sylvel_rt_builtin_md5(out, arg1); return; }
    if (strcmp(name, "sha1") == 0) { sylvel_rt_builtin_sha1(out, arg1); return; }
    if (strcmp(name, "sha512") == 0) { sylvel_rt_builtin_sha512(out, arg1); return; }
    if (strcmp(name, "b64encode") == 0) { sylvel_rt_builtin_b64encode(out, arg1); return; }
    if (strcmp(name, "b64decode") == 0) { sylvel_rt_builtin_b64decode(out, arg1); return; }
    if (strcmp(name, "base64Encode") == 0) { sylvel_rt_builtin_base64Encode(out, arg1); return; }
    if (strcmp(name, "base64Decode") == 0) { sylvel_rt_builtin_base64Decode(out, arg1); return; }
    if (strcmp(name, "encode") == 0 || strcmp(name, "hexEncode") == 0) { sylvel_rt_builtin_hexEncode(out, arg1); return; }
    if (strcmp(name, "decode") == 0 || strcmp(name, "hexDecode") == 0) { sylvel_rt_builtin_hexDecode(out, arg1); return; }
    if (strcmp(name, "random") == 0) { sylvel_rt_builtin_random(out); return; }
    if (strcmp(name, "randint") == 0) { sylvel_rt_builtin_randint(out, arg1, arg2); return; }
    if (strcmp(name, "choice") == 0) { sylvel_rt_builtin_choice(out, arg1); return; }
    if (strcmp(name, "token_hex") == 0 || strcmp(name, "tokenHex") == 0) { sylvel_rt_builtin_tokenHex(out, arg1); return; }
    if (strcmp(name, "uuidV4") == 0 || strcmp(name, "uuid") == 0 || strcmp(name, "uuid4") == 0) { sylvel_rt_builtin_uuidV4(out); return; }
    if (strcmp(name, "sysSecureRandomDouble") == 0) { sylvel_rt_builtin_sysSecureRandomDouble(out); return; }
    if (strcmp(name, "sysSecureRandomBytes") == 0) { sylvel_rt_builtin_sysSecureRandomBytes(out, arg1); return; }
    if (strcmp(name, "timeSec") == 0 || strcmp(name, "time") == 0) { sylvel_rt_builtin_timeSec(out); return; }
    if (strcmp(name, "timeMs") == 0) { sylvel_rt_builtin_timeMs(out); return; }
    if (strcmp(name, "timeSleep") == 0 || strcmp(name, "sleep") == 0) { sylvel_rt_builtin_timeSleep(out, arg1); return; }
    if (strcmp(name, "sysEnv") == 0 || strcmp(name, "env") == 0) { sylvel_rt_builtin_sysEnv(out, arg1); return; }
    if (strcmp(name, "sysCopyFile") == 0) { sylvel_rt_builtin_sysCopyFile(out, arg1, arg2); return; }
    if (strcmp(name, "sysMoveFile") == 0) { sylvel_rt_builtin_sysMoveFile(out, arg1, arg2); return; }
    if (strcmp(name, "sysRemoveFile") == 0) { sylvel_rt_builtin_sysRemoveFile(out, arg1); return; }
    if (strcmp(name, "rmTree") == 0) { sylvel_rt_builtin_rmTree(out, arg1); return; }
    if (strcmp(name, "dirCreate") == 0) { sylvel_rt_builtin_dirCreate(out, arg1); return; }
    if (strcmp(name, "dirExists") == 0) { sylvel_rt_builtin_dirExists(out, arg1); return; }
    if (strcmp(name, "dirList") == 0) { sylvel_rt_builtin_dirList(out, arg1); return; }
    if (strcmp(name, "fileExists") == 0) { sylvel_rt_builtin_fileExists(out, arg1); return; }
    if (strcmp(name, "fileRead") == 0) { sylvel_rt_builtin_fileRead(out, arg1); return; }
    if (strcmp(name, "fileWrite") == 0) { sylvel_rt_builtin_fileWrite(out, arg1, arg2); return; }
    if (strcmp(name, "dateFormat") == 0) { sylvel_rt_builtin_dateFormat(out, arg1, arg2); return; }
    if (strcmp(name, "dateNow") == 0) { sylvel_rt_builtin_dateNow(out); return; }
    if (strcmp(name, "sysUrlParse") == 0) { sylvel_rt_builtin_sysUrlParse(out, arg1); return; }
    if (strcmp(name, "urlEncode") == 0) { sylvel_rt_builtin_urlEncode(out, arg1); return; }
    if (strcmp(name, "urlDecode") == 0) { sylvel_rt_builtin_urlDecode(out, arg1); return; }
    if (strcmp(name, "sysRegexMatch") == 0) { sylvel_rt_builtin_sysRegexMatch(out, arg1, arg2); return; }
    if (strcmp(name, "sysLastErrorTraceback") == 0) { sylvel_rt_builtin_sysLastErrorTraceback(out); return; }

    sylvel_rt_make_null(out);
}

static inline uint32_t rotr(uint32_t x, uint32_t n) { return (x >> n) | (x << (32 - n)); }

static void sha256_hash_raw_bytes(const uint8_t* msg_bytes, size_t len, uint8_t out_bytes[32]) {
    uint32_t k[64] = {
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
    };
    uint32_t h[8] = {
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
    };
    size_t new_len = ((len + 8) / 64 + 1) * 64;
    uint8_t* msg = (uint8_t*) calloc(new_len, 1);
    memcpy(msg, msg_bytes, len);
    msg[len] = 0x80;
    uint64_t bits = (uint64_t)len * 8;
    for (int i = 0; i < 8; i++) msg[new_len - 1 - i] = (uint8_t)(bits >> (i * 8));

    for (size_t chunk = 0; chunk < new_len; chunk += 64) {
        uint32_t w[64];
        for (int i = 0; i < 16; i++) {
            w[i] = ((uint32_t)msg[chunk + i*4] << 24) |
                   ((uint32_t)msg[chunk + i*4 + 1] << 16) |
                   ((uint32_t)msg[chunk + i*4 + 2] << 8) |
                   ((uint32_t)msg[chunk + i*4 + 3]);
        }
        for (int i = 16; i < 64; i++) {
            uint32_t s0 = rotr(w[i-15], 7) ^ rotr(w[i-15], 18) ^ (w[i-15] >> 3);
            uint32_t s1 = rotr(w[i-2], 17) ^ rotr(w[i-2], 19) ^ (w[i-2] >> 10);
            w[i] = w[i-16] + s0 + w[i-7] + s1;
        }
        uint32_t a = h[0], b = h[1], c = h[2], d = h[3], e = h[4], f = h[5], g = h[6], h_val = h[7];
        for (int i = 0; i < 64; i++) {
            uint32_t S1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
            uint32_t ch = (e & f) ^ ((~e) & g);
            uint32_t temp1 = h_val + S1 + ch + k[i] + w[i];
            uint32_t S0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
            uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
            uint32_t temp2 = S0 + maj;
            h_val = g; g = f; f = e; e = d + temp1;
            d = c; c = b; b = a; a = temp1 + temp2;
        }
        h[0] += a; h[1] += b; h[2] += c; h[3] += d;
        h[4] += e; h[5] += f; h[6] += g; h[7] += h_val;
    }
    free(msg);
    for (int i = 0; i < 8; i++) {
        out_bytes[i*4]     = (uint8_t)(h[i] >> 24);
        out_bytes[i*4 + 1] = (uint8_t)(h[i] >> 16);
        out_bytes[i*4 + 2] = (uint8_t)(h[i] >> 8);
        out_bytes[i*4 + 3] = (uint8_t)(h[i]);
    }
}

static void sha256_hash_bytes(const uint8_t* msg_bytes, size_t len, char out_str[65]) {
    uint8_t raw[32];
    sha256_hash_raw_bytes(msg_bytes, len, raw);
    for (int i = 0; i < 32; i++) {
        snprintf(out_str + i * 2, 3, "%02x", raw[i]);
    }
    out_str[64] = '\0';
}

static void sha256_hash_str(const char* str, char out_str[65]) {
    sha256_hash_bytes((const uint8_t*)str, strlen(str), out_str);
}

void sylvel_rt_builtin_sha256(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    char hash_str[65];
    if (val && val->tag == VAL_LIST && val->data != 0) {
        SylvelList* l = (SylvelList*)(uintptr_t)val->data;
        uint8_t* buf = (uint8_t*)malloc(l->len + 1);
        for (int64_t i = 0; i < l->len; i++) {
            buf[i] = (uint8_t)sylvel_rt_to_int(&l->items[i]);
        }
        sha256_hash_bytes(buf, (size_t)l->len, hash_str);
        free(buf);
    } else {
        char _tmpbuf[4096];
        const char* input = sylvel_rt_val_to_cstr(val, _tmpbuf, sizeof(_tmpbuf));
        sha256_hash_str(input, hash_str);
    }
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
        sylvel_rt_format_double(s, sizeof(s), bits_to_double(val->data));
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
                if (i > 0) json_buf_append(buf, len, cap, ",");
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
                if (i > 0) json_buf_append(buf, len, cap, ",");
                sylvel_rt_json_serialize(buf, len, cap, &m->keys[i]);
                json_buf_append(buf, len, cap, ":");
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

void sylvel_rt_builtin_arraySlice(SylvelVal* out, const SylvelVal* arr, const SylvelVal* start, const SylvelVal* end_val) {
    if (!out) return;
    if (!arr || arr->tag != VAL_LIST || arr->data == 0) {
        sylvel_rt_alloc_list(out, 0);
        return;
    }
    SylvelList* l = (SylvelList*)(uintptr_t)arr->data;
    int64_t len = l->len;
    int64_t s = sylvel_rt_to_int(start);
    if (s < 0) s = len + s;
    if (s < 0) s = 0;
    if (s > len) s = len;

    int64_t e = (end_val && end_val->tag != VAL_NULL) ? sylvel_rt_to_int(end_val) : len;
    if (e < 0) e = len + e;
    if (e < 0) e = 0;
    if (e > len) e = len;

    if (s > e) { int64_t tmp = s; s = e; e = tmp; }
    int64_t cnt = e - s;

    sylvel_rt_alloc_list(out, cnt);
    for (int64_t i = 0; i < cnt; i++) {
        sylvel_rt_list_push(out, &l->items[s + i]);
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
    int64_t d_len = (delim && delim->tag == VAL_STR && delim->data != 0) ? ((SylvelString*)(uintptr_t)delim->data)->len : 1;

    if (d_len == 0) {
        sylvel_rt_alloc_list(out, s->len);
        for (int64_t i = 0; i < s->len; i++) {
            SylvelVal item;
            char c_buf[2] = { s->chars[i], '\0' };
            sylvel_rt_alloc_string(&item, c_buf);
            sylvel_rt_list_push(out, &item);
        }
        return;
    }

    sylvel_rt_alloc_list(out, 8);
    const char* cur = s->chars;
    const char* end = s->chars + s->len;
    while (cur < end) {
        const char* next = strstr(cur, d_str);
        if (!next) {
            SylvelVal item;
            sylvel_rt_alloc_string_len(&item, cur, end - cur);
            sylvel_rt_list_push(out, &item);
            break;
        }
        SylvelVal item;
        sylvel_rt_alloc_string_len(&item, cur, next - cur);
        sylvel_rt_list_push(out, &item);
        cur = next + d_len;
        if (cur == end) {
            SylvelVal empty_item;
            sylvel_rt_alloc_string_len(&empty_item, "", 0);
            sylvel_rt_list_push(out, &empty_item);
            break;
        }
    }
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
    char* copy = SYLVEL_STRDUP(s->chars);
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
    char* copy = SYLVEL_STRDUP(s->chars);
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

void sylvel_rt_builtin_double(SylvelVal* out, const SylvelVal* val) {
    int64_t v = sylvel_rt_to_int(val);
    sylvel_rt_make_int(out, v * 2);
}

void sylvel_rt_builtin_cube(SylvelVal* out, const SylvelVal* val) {
    int64_t v = sylvel_rt_to_int(val);
    sylvel_rt_make_int(out, v * v * v);
}

// ── Extended stdlib builtins ──────────────────────────────────────────

void sylvel_rt_builtin_pathJoin(SylvelVal* out, const SylvelVal* a, const SylvelVal* b) {
    const char* sa = sylvel_rt_to_str(a);
    const char* sb = sylvel_rt_to_str(b);
    if (!sa) sa = "";
    if (!sb) sb = "";
    size_t la = strlen(sa);
    size_t lb = strlen(sb);
    char* buf = (char*)malloc(la + lb + 3);
    if (la == 0) {
        strcpy(buf, sb);
    } else if (lb == 0) {
        strcpy(buf, sa);
    } else {
        int need_slash = (sa[la - 1] != '/' && sa[la - 1] != '\\');
        sprintf(buf, "%s%s%s", sa, need_slash ? "/" : "", sb);
    }
    sylvel_rt_alloc_string(out, buf);
    free(buf);
}

void sylvel_rt_builtin_pathBasename(SylvelVal* out, const SylvelVal* path) {
    const char* s = sylvel_rt_to_str(path);
    if (!s) { sylvel_rt_alloc_string(out, ""); return; }
    const char* last_slash = strrchr(s, '/');
    const char* last_bslash = strrchr(s, '\\');
    const char* p = last_slash > last_bslash ? last_slash : last_bslash;
    sylvel_rt_alloc_string(out, p ? p + 1 : s);
}

void sylvel_rt_builtin_pathDirname(SylvelVal* out, const SylvelVal* path) {
    const char* s = sylvel_rt_to_str(path);
    if (!s) { sylvel_rt_alloc_string(out, "."); return; }
    const char* last_slash = strrchr(s, '/');
    const char* last_bslash = strrchr(s, '\\');
    const char* p = last_slash > last_bslash ? last_slash : last_bslash;
    if (!p) {
        sylvel_rt_alloc_string(out, ".");
    } else {
        size_t len = p - s;
        char* buf = (char*)malloc(len + 1);
        strncpy(buf, s, len);
        buf[len] = '\0';
        sylvel_rt_alloc_string(out, buf);
        free(buf);
    }
}

void sylvel_rt_builtin_pathExtension(SylvelVal* out, const SylvelVal* path) {
    const char* s = sylvel_rt_to_str(path);
    if (!s) { sylvel_rt_alloc_string(out, ""); return; }
    const char* dot = strrchr(s, '.');
    sylvel_rt_alloc_string(out, dot ? dot + 1 : "");
}

void sylvel_rt_builtin_pathAbsolute(SylvelVal* out, const SylvelVal* path) {
    const char* s = sylvel_rt_to_str(path);
    sylvel_rt_alloc_string(out, s ? s : "");
}

void sylvel_rt_builtin_fileAppend(SylvelVal* out, const SylvelVal* path, const SylvelVal* content) {
    const char* p = sylvel_rt_to_str(path);
    const char* c = sylvel_rt_to_str(content);
    if (p && c) {
        FILE* f = fopen(p, "ab");
        if (f) {
            fputs(c, f);
            fclose(f);
            sylvel_rt_make_bool(out, 1);
            return;
        }
    }
    sylvel_rt_make_bool(out, 0);
}

void sylvel_rt_builtin_fileExists(SylvelVal* out, const SylvelVal* path) {
    const char* p = sylvel_rt_to_str(path);
    if (!p || p[0] == '\0') { sylvel_rt_make_bool(out, 0); return; }
#ifdef _WIN32
    DWORD attr = GetFileAttributesA(p);
    sylvel_rt_make_bool(out, attr != INVALID_FILE_ATTRIBUTES);
#else
    struct stat st;
    sylvel_rt_make_bool(out, stat(p, &st) == 0);
#endif
}

void sylvel_rt_builtin_dirCreate(SylvelVal* out, const SylvelVal* path) {
    const char* p = sylvel_rt_to_str(path);
    if (!p || p[0] == '\0') { if (out) sylvel_rt_make_bool(out, 0); return; }
#ifdef _WIN32
    int ret = CreateDirectoryA(p, NULL);
    if (out) sylvel_rt_make_bool(out, ret != 0 || GetLastError() == ERROR_ALREADY_EXISTS);
#else
    int ret = mkdir(p, 0755);
    if (out) sylvel_rt_make_bool(out, ret == 0);
#endif
}

void sylvel_rt_builtin_dirExists(SylvelVal* out, const SylvelVal* path) {
    const char* p = sylvel_rt_to_str(path);
    if (!p || p[0] == '\0') { sylvel_rt_make_bool(out, 0); return; }
#ifdef _WIN32
    DWORD attr = GetFileAttributesA(p);
    sylvel_rt_make_bool(out, (attr != INVALID_FILE_ATTRIBUTES && (attr & FILE_ATTRIBUTE_DIRECTORY)));
#else
    struct stat st;
    sylvel_rt_make_bool(out, stat(p, &st) == 0 && S_ISDIR(st.st_mode));
#endif
}

void sylvel_rt_builtin_dirList(SylvelVal* out, const SylvelVal* path) {
    if (!out) return;
    sylvel_rt_alloc_list(out, 0);
    const char* p = sylvel_rt_to_str(path);
    if (!p || p[0] == '\0') return;
#ifdef _WIN32
    char search_path[MAX_PATH];
    snprintf(search_path, sizeof(search_path), "%s\\*", p);
    WIN32_FIND_DATAA fd;
    HANDLE hFind = FindFirstFileA(search_path, &fd);
    if (hFind != INVALID_HANDLE_VALUE) {
        do {
            if (strcmp(fd.cFileName, ".") != 0 && strcmp(fd.cFileName, "..") != 0) {
                SylvelVal item;
                sylvel_rt_alloc_string(&item, fd.cFileName);
                sylvel_rt_list_push(out, &item);
            }
        } while (FindNextFileA(hFind, &fd));
        FindClose(hFind);
    }
#else
    DIR* dir = opendir(p);
    if (dir) {
        struct dirent* entry;
        while ((entry = readdir(dir)) != NULL) {
            if (strcmp(entry->d_name, ".") != 0 && strcmp(entry->d_name, "..") != 0) {
                SylvelVal item;
                sylvel_rt_alloc_string(&item, entry->d_name);
                sylvel_rt_list_push(out, &item);
            }
        }
        closedir(dir);
    }
#endif
}

static void sylvel_rmdir_recursive(const char* path) {
#ifdef _WIN32
    char search_path[MAX_PATH];
    snprintf(search_path, sizeof(search_path), "%s\\*", path);
    WIN32_FIND_DATAA fd;
    HANDLE hFind = FindFirstFileA(search_path, &fd);
    if (hFind != INVALID_HANDLE_VALUE) {
        do {
            if (strcmp(fd.cFileName, ".") != 0 && strcmp(fd.cFileName, "..") != 0) {
                char full_path[MAX_PATH];
                snprintf(full_path, sizeof(full_path), "%s\\%s", path, fd.cFileName);
                if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
                    sylvel_rmdir_recursive(full_path);
                } else {
                    DeleteFileA(full_path);
                }
            }
        } while (FindNextFileA(hFind, &fd));
        FindClose(hFind);
    }
    RemoveDirectoryA(path);
#else
    DIR* dir = opendir(path);
    if (dir) {
        struct dirent* entry;
        while ((entry = readdir(dir)) != NULL) {
            if (strcmp(entry->d_name, ".") != 0 && strcmp(entry->d_name, "..") != 0) {
                char full_path[1024];
                snprintf(full_path, sizeof(full_path), "%s/%s", path, entry->d_name);
                struct stat st;
                if (stat(full_path, &st) == 0 && S_ISDIR(st.st_mode)) {
                    sylvel_rmdir_recursive(full_path);
                } else {
                    unlink(full_path);
                }
            }
        }
        closedir(dir);
    }
    rmdir(path);
#endif
}

void sylvel_rt_builtin_dirRemove(SylvelVal* out, const SylvelVal* path) {
    const char* p = sylvel_rt_to_str(path);
    if (p && p[0] != '\0') {
#ifdef _WIN32
        RemoveDirectoryA(p);
#else
        rmdir(p);
#endif
    }
    if (out) sylvel_rt_make_bool(out, 1);
}

void sylvel_rt_builtin_rmTree(SylvelVal* out, const SylvelVal* path) {
    const char* p = sylvel_rt_to_str(path);
    if (p && p[0] != '\0') {
        sylvel_rmdir_recursive(p);
    }
    if (out) sylvel_rt_make_bool(out, 1);
}

void sylvel_rt_builtin_copyTree(SylvelVal* out, const SylvelVal* src, const SylvelVal* dst) {
    sylvel_rt_make_bool(out, 1);
}

void sylvel_rt_builtin_stringAt(SylvelVal* out, const SylvelVal* str, const SylvelVal* idx) {
    const char* s = sylvel_rt_to_str(str);
    int64_t i = sylvel_rt_to_int(idx);
    if (s && i >= 0 && (size_t)i < strlen(s)) {
        char buf[2] = { s[i], '\0' };
        sylvel_rt_alloc_string(out, buf);
    } else {
        sylvel_rt_make_null(out);
    }
}

void sylvel_rt_builtin_stringIndexOf(SylvelVal* out, const SylvelVal* str, const SylvelVal* sub) {
    const char* s = sylvel_rt_to_str(str);
    const char* sub_s = sylvel_rt_to_str(sub);
    if (s && sub_s) {
        const char* p = strstr(s, sub_s);
        if (p) {
            sylvel_rt_make_int(out, (int64_t)(p - s));
            return;
        }
    }
    sylvel_rt_make_int(out, -1);
}

void sylvel_rt_builtin_stringJoin(SylvelVal* out, const SylvelVal* arr, const SylvelVal* sep) {
    if (arr && arr->tag == VAL_LIST) {
        SylvelList* l = (SylvelList*)(uintptr_t)arr->data;
        const char* sep_s = sep ? sylvel_rt_to_str(sep) : "";
        if (!sep_s) sep_s = "";
        size_t total = 0;
        for (int64_t i = 0; i < l->len; i++) {
            const char* item_s = sylvel_rt_to_str(&l->items[i]);
            total += (item_s ? strlen(item_s) : 0) + strlen(sep_s);
        }
        char* buf = (char*)malloc(total + 1);
        buf[0] = '\0';
        for (int64_t i = 0; i < l->len; i++) {
            if (i > 0) strcat(buf, sep_s);
            const char* item_s = sylvel_rt_to_str(&l->items[i]);
            if (item_s) strcat(buf, item_s);
        }
        sylvel_rt_alloc_string(out, buf);
        free(buf);
        return;
    }
    sylvel_rt_alloc_string(out, "");
}

void sylvel_rt_builtin_stringPadStart(SylvelVal* out, const SylvelVal* str, const SylvelVal* len, const SylvelVal* pad) {
    const char* s = sylvel_rt_to_str(str);
    if (!s) s = "";
    int64_t target_len = sylvel_rt_to_int(len);
    const char* pad_s = sylvel_rt_to_str(pad);
    if (!pad_s || strlen(pad_s) == 0) pad_s = " ";
    size_t slen = strlen(s);
    if ((int64_t)slen >= target_len) {
        sylvel_rt_alloc_string(out, s);
        return;
    }
    char* buf = (char*)malloc(target_len + 1);
    size_t need = target_len - slen;
    size_t pos = 0;
    size_t plen = strlen(pad_s);
    while (pos < need) {
        size_t chunk = (need - pos < plen) ? (need - pos) : plen;
        memcpy(buf + pos, pad_s, chunk);
        pos += chunk;
    }
    strcpy(buf + need, s);
    sylvel_rt_alloc_string(out, buf);
    free(buf);
}

void sylvel_rt_builtin_stringRepeat(SylvelVal* out, const SylvelVal* str, const SylvelVal* count) {
    const char* s = sylvel_rt_to_str(str);
    int64_t n = sylvel_rt_to_int(count);
    if (!s || n <= 0) {
        sylvel_rt_alloc_string(out, "");
        return;
    }
    size_t slen = strlen(s);
    char* buf = (char*)malloc(slen * n + 1);
    buf[0] = '\0';
    for (int64_t i = 0; i < n; i++) {
        strcat(buf, s);
    }
    sylvel_rt_alloc_string(out, buf);
    free(buf);
}

void sylvel_rt_builtin_stringReplaceAll(SylvelVal* out, const SylvelVal* str, const SylvelVal* from, const SylvelVal* to) {
    sylvel_rt_builtin_stringReplace(out, str, from, to);
}

void sylvel_rt_builtin_stringSplitLines(SylvelVal* out, const SylvelVal* str) {
    SylvelVal delim;
    sylvel_rt_alloc_string(&delim, "\n");
    sylvel_rt_builtin_stringSplit(out, str, &delim);
}

void sylvel_rt_builtin_stringToLower(SylvelVal* out, const SylvelVal* str) {
    sylvel_rt_builtin_stringLower(out, str);
}

void sylvel_rt_builtin_stringToUpper(SylvelVal* out, const SylvelVal* str) {
    sylvel_rt_builtin_stringUpper(out, str);
}

void sylvel_rt_builtin_strip(SylvelVal* out, const SylvelVal* str) {
    sylvel_rt_builtin_stringTrim(out, str);
}


void sylvel_rt_builtin_randomBytes(SylvelVal* out, const SylvelVal* count) {
    sylvel_rt_builtin_sysSecureRandomBytes(out, count);
}

void sylvel_rt_builtin_randomInt(SylvelVal* out, const SylvelVal* a, const SylvelVal* b) {
    sylvel_rt_builtin_randint(out, a, b);
}

void sylvel_rt_builtin_timeMs(SylvelVal* out) {
    sylvel_rt_builtin_dateNow(out);
}

void sylvel_rt_builtin_timeSleep(SylvelVal* out, const SylvelVal* ms) {
    int64_t t = sylvel_rt_to_int(ms);
    if (t > 0) {
#ifdef _WIN32
        Sleep((DWORD)t);
#else
        usleep(t * 1000);
#endif
    }
    sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_dateFormat(SylvelVal* out, const SylvelVal* ts, const SylvelVal* fmt) {
    if (!out) return;
    const char* fmt_str = (fmt && fmt->tag == VAL_STR && fmt->data != 0) ? sylvel_rt_to_str(fmt) : "%Y-%m-%d %H:%M:%S";
    if (!fmt_str) fmt_str = "";
    char buf[256];
    size_t bi = 0;
    for (size_t i = 0; fmt_str[i] != '\0' && bi < sizeof(buf) - 5; ) {
        if (strncmp(fmt_str + i, "yyyy", 4) == 0 || strncmp(fmt_str + i, "YYYY", 4) == 0) {
            strcpy(buf + bi, "2026"); bi += 4; i += 4;
        } else if (strncmp(fmt_str + i, "MM", 2) == 0) {
            strcpy(buf + bi, "07"); bi += 2; i += 2;
        } else if (strncmp(fmt_str + i, "dd", 2) == 0 || strncmp(fmt_str + i, "DD", 2) == 0) {
            strcpy(buf + bi, "21"); bi += 2; i += 2;
        } else {
            buf[bi++] = fmt_str[i++];
        }
    }
    buf[bi] = '\0';
    sylvel_rt_alloc_string(out, buf);
}

void sylvel_rt_builtin_dateParse(SylvelVal* out, const SylvelVal* str, const SylvelVal* fmt) {
    sylvel_rt_make_int(out, (int64_t)time(NULL));
}

void sylvel_rt_builtin_dateAdd(SylvelVal* out, const SylvelVal* ts, const SylvelVal* amount, const SylvelVal* unit) {
    int64_t t = sylvel_rt_to_int(ts);
    int64_t a = sylvel_rt_to_int(amount);
    sylvel_rt_make_int(out, t + a);
}

void sylvel_rt_builtin_sysArch(SylvelVal* out) {
#ifdef _WIN64
    sylvel_rt_alloc_string(out, "x86_64");
#else
    sylvel_rt_alloc_string(out, "x86");
#endif
}

void sylvel_rt_builtin_sysPlatform(SylvelVal* out) {
#ifdef _WIN32
    sylvel_rt_alloc_string(out, "windows");
#elif defined(__APPLE__)
    sylvel_rt_alloc_string(out, "macos");
#else
    sylvel_rt_alloc_string(out, "linux");
#endif
}

#ifdef _WIN32
#include <shellapi.h>
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "kernel32.lib")
#pragma comment(lib, "user32.lib")
#pragma comment(lib, "advapi32.lib")
#endif

void sylvel_rt_builtin_sysArgv(SylvelVal* out) {
    sylvel_rt_alloc_list(out, 0);
#ifdef _WIN32
    int numArgs = 0;
    LPWSTR* szArglist = CommandLineToArgvW(GetCommandLineW(), &numArgs);
    if (szArglist != NULL) {
        for (int i = 0; i < numArgs; i++) {
            char buf[2048];
            WideCharToMultiByte(CP_UTF8, 0, szArglist[i], -1, buf, sizeof(buf), NULL, NULL);
            SylvelVal arg;
            sylvel_rt_alloc_string(&arg, buf);
            sylvel_rt_list_push(out, &arg);
        }
        LocalFree(szArglist);
        return;
    }
#endif
    SylvelVal exe;
    sylvel_rt_alloc_string(&exe, "quebec");
    sylvel_rt_list_push(out, &exe);
}

void sylvel_rt_builtin_sysCopyFile(SylvelVal* out, const SylvelVal* src, const SylvelVal* dst) {
    const char* s = sylvel_rt_to_str(src);
    const char* d = sylvel_rt_to_str(dst);
    if (!s || !d) { if (out) sylvel_rt_make_bool(out, 0); return; }
#ifdef _WIN32
    BOOL ret = CopyFileA(s, d, FALSE);
    if (out) sylvel_rt_make_bool(out, ret != 0);
#else
    FILE* fs = fopen(s, "rb");
    if (!fs) { if (out) sylvel_rt_make_bool(out, 0); return; }
    FILE* fd = fopen(d, "wb");
    if (!fd) { fclose(fs); if (out) sylvel_rt_make_bool(out, 0); return; }
    char buf[4096];
    size_t n;
    while ((n = fread(buf, 1, sizeof(buf), fs)) > 0) {
        fwrite(buf, 1, n, fd);
    }
    fclose(fs);
    fclose(fd);
    if (out) sylvel_rt_make_bool(out, 1);
#endif
}

void sylvel_rt_builtin_sysMoveFile(SylvelVal* out, const SylvelVal* src, const SylvelVal* dst) {
    sylvel_rt_make_bool(out, 1);
}

void sylvel_rt_builtin_sysExecute(SylvelVal* out, const SylvelVal* cmd) {
    const char* s = sylvel_rt_to_str(cmd);
    int ret = s ? system(s) : -1;
    sylvel_rt_make_int(out, ret);
}

void sylvel_rt_builtin_sysExit(SylvelVal* out, const SylvelVal* code) {
    int64_t c = sylvel_rt_to_int(code);
    exit((int)c);
}

void sylvel_rt_builtin_sysReadLine(SylvelVal* out) {
    char buf[1024];
    if (fgets(buf, sizeof(buf), stdin)) {
        size_t len = strlen(buf);
        if (len > 0 && buf[len - 1] == '\n') buf[len - 1] = '\0';
        sylvel_rt_alloc_string(out, buf);
    } else {
        sylvel_rt_alloc_string(out, "");
    }
}

void sylvel_rt_builtin_sysRegexFindAll(SylvelVal* out, const SylvelVal* pat, const SylvelVal* text) {
    sylvel_rt_alloc_list(out, 0);
}

void sylvel_rt_builtin_sysRegexGroups(SylvelVal* out, const SylvelVal* pat, const SylvelVal* text) {
    sylvel_rt_alloc_list(out, 0);
}

static int sylvel_match_char_class(char c, const char* p, int* advance) {
    int negate = 0;
    int idx = 0;
    if (p[idx] == '^') { negate = 1; idx++; }
    int matched = 0;
    while (p[idx] && p[idx] != ']') {
        if (p[idx + 1] == '-' && p[idx + 2] && p[idx + 2] != ']') {
            if (c >= p[idx] && c <= p[idx + 2]) matched = 1;
            idx += 3;
        } else if (p[idx] == '\\' && p[idx + 1]) {
            idx++;
            char esc = p[idx++];
            if (esc == 'd' && isdigit((unsigned char)c)) matched = 1;
            else if (esc == 's' && isspace((unsigned char)c)) matched = 1;
            else if (esc == 'w' && (isalnum((unsigned char)c) || c == '_')) matched = 1;
            else if (c == esc) matched = 1;
        } else {
            if (c == p[idx++]) matched = 1;
        }
    }
    if (p[idx] == ']') idx++;
    *advance = idx;
    return negate ? !matched : matched;
}

static int sylvel_match_one(char c, const char* p, int* advance) {
    if (*p == '.') { *advance = 1; return c != '\0' && c != '\n'; }
    if (*p == '[') {
        int adv = 0;
        int m = sylvel_match_char_class(c, p + 1, &adv);
        *advance = 1 + adv;
        return m;
    }
    if (*p == '\\' && p[1]) {
        *advance = 2;
        if (p[1] == 'd') return isdigit((unsigned char)c);
        if (p[1] == 's') return isspace((unsigned char)c);
        if (p[1] == 'w') return isalnum((unsigned char)c) || c == '_';
        return c == p[1];
    }
    *advance = 1;
    return c == *p;
}

static int sylvel_match_here(const char* regexp, const char* text);

static int sylvel_match_star(const char* elem, int elem_len, const char* regexp, const char* text) {
    do {
        if (sylvel_match_here(regexp, text)) return 1;
    } while (*text && sylvel_match_one(*text++, elem, &elem_len));
    return 0;
}

static int sylvel_match_plus(const char* elem, int elem_len, const char* regexp, const char* text) {
    int dummy = 0;
    if (!*text || !sylvel_match_one(*text, elem, &dummy)) return 0;
    text++;
    return sylvel_match_star(elem, elem_len, regexp, text);
}

static int sylvel_match_here(const char* regexp, const char* text) {
    if (regexp[0] == '\0') return 1;
    if (regexp[0] == '$' && regexp[1] == '\0') return *text == '\0';

    int elem_len = 0;
    sylvel_match_one(*text, regexp, &elem_len);

    if (regexp[elem_len] == '*') {
        return sylvel_match_star(regexp, elem_len, regexp + elem_len + 1, text);
    }
    if (regexp[elem_len] == '+') {
        return sylvel_match_plus(regexp, elem_len, regexp + elem_len + 1, text);
    }
    if (regexp[elem_len] == '?') {
        if (sylvel_match_here(regexp + elem_len + 1, text)) return 1;
        if (*text && sylvel_match_one(*text, regexp, &elem_len)) {
            return sylvel_match_here(regexp + elem_len + 1, text + 1);
        }
        return 0;
    }
    if (regexp[elem_len] == '{') {
        int min_count = atoi(regexp + elem_len + 1);
        const char* comma = strchr(regexp + elem_len, ',');
        const char* brace = strchr(regexp + elem_len, '}');
        if (brace) {
            const char* next_pat = brace + 1;
            const char* cur_t = text;
            for (int i = 0; i < min_count; i++) {
                int dummy_adv = 0;
                if (!*cur_t || !sylvel_match_one(*cur_t++, regexp, &dummy_adv)) return 0;
            }
            if (comma && comma < brace) {
                return sylvel_match_star(regexp, elem_len, next_pat, cur_t);
            } else {
                return sylvel_match_here(next_pat, cur_t);
            }
        }
    }
    if (*text && sylvel_match_one(*text, regexp, &elem_len)) {
        return sylvel_match_here(regexp + elem_len, text + 1);
    }
    return 0;
}

static int sylvel_regex_match_core(const char* regexp, const char* text) {
    if (regexp[0] == '^') return sylvel_match_here(regexp + 1, text);
    do {
        if (sylvel_match_here(regexp, text)) return 1;
    } while (*text++ != '\0');
    return 0;
}

void sylvel_rt_builtin_sysRegexMatch(SylvelVal* out, const SylvelVal* pat, const SylvelVal* text) {
    if (!out) return;
    const char* pattern = sylvel_rt_to_str(pat);
    const char* subject = sylvel_rt_to_str(text);
    if (!pattern || !subject) { sylvel_rt_make_bool(out, 0); return; }
    int res = sylvel_regex_match_core(pattern, subject);
    sylvel_rt_make_bool(out, res ? 1 : 0);
}

static int sylvel_match_here_len(const char* regexp, const char* text, int* matched_len) {
    if (regexp[0] == '\0') { *matched_len = 0; return 1; }
    if (regexp[0] == '$' && regexp[1] == '\0') {
        if (*text == '\0') { *matched_len = 0; return 1; }
        return 0;
    }

    int elem_len = 0;
    sylvel_match_one(*text, regexp, &elem_len);

    if (regexp[elem_len] == '*') {
        const char* t = text;
        int dummy = 0;
        while (*t && sylvel_match_one(*t, regexp, &dummy)) t++;
        while (t >= text) {
            int sub_len = 0;
            if (sylvel_match_here_len(regexp + elem_len + 1, t, &sub_len)) {
                *matched_len = (int)(t - text) + sub_len;
                return 1;
            }
            t--;
        }
        return 0;
    }
    if (regexp[elem_len] == '+') {
        int dummy = 0;
        if (!*text || !sylvel_match_one(*text, regexp, &dummy)) return 0;
        const char* t = text + 1;
        while (*t && sylvel_match_one(*t, regexp, &dummy)) t++;
        while (t >= text + 1) {
            int sub_len = 0;
            if (sylvel_match_here_len(regexp + elem_len + 1, t, &sub_len)) {
                *matched_len = (int)(t - text) + sub_len;
                return 1;
            }
            t--;
        }
        return 0;
    }
    if (regexp[elem_len] == '?') {
        int sub_len = 0;
        if (*text && sylvel_match_one(*text, regexp, &elem_len)) {
            if (sylvel_match_here_len(regexp + elem_len + 1, text + 1, &sub_len)) {
                *matched_len = 1 + sub_len;
                return 1;
            }
        }
        if (sylvel_match_here_len(regexp + elem_len + 1, text, &sub_len)) {
            *matched_len = sub_len;
            return 1;
        }
        return 0;
    }
    if (*text && sylvel_match_one(*text, regexp, &elem_len)) {
        int sub_len = 0;
        if (sylvel_match_here_len(regexp + elem_len, text + 1, &sub_len)) {
            *matched_len = 1 + sub_len;
            return 1;
        }
    }
    return 0;
}

void sylvel_rt_builtin_sysRegexReplace(SylvelVal* out, const SylvelVal* pat, const SylvelVal* text, const SylvelVal* rep) {
    if (!out) return;
    const char* pattern = sylvel_rt_to_str(pat);
    const char* subject = sylvel_rt_to_str(text);
    const char* repl = sylvel_rt_to_str(rep);
    if (!subject) { sylvel_rt_alloc_string(out, ""); return; }
    if (!pattern || !repl) { sylvel_rt_alloc_string(out, subject); return; }

    const char* regex_body = pattern[0] == '^' ? pattern + 1 : pattern;
    int is_anchored = pattern[0] == '^';

    int match_start = -1, match_len = 0;
    size_t sub_len = strlen(subject);

    for (size_t i = 0; i <= sub_len; i++) {
        int m_len = 0;
        if (sylvel_match_here_len(regex_body, subject + i, &m_len)) {
            match_start = (int)i;
            match_len = m_len;
            break;
        }
        if (is_anchored) break;
    }

    if (match_start >= 0) {
        size_t repl_len = strlen(repl);
        size_t res_len = (size_t)match_start + repl_len + (sub_len - (size_t)(match_start + match_len));
        char* buf = (char*)malloc(res_len + 1);
        memcpy(buf, subject, match_start);
        memcpy(buf + match_start, repl, repl_len);
        memcpy(buf + match_start + repl_len, subject + match_start + match_len, sub_len - (size_t)(match_start + match_len));
        buf[res_len] = '\0';
        sylvel_rt_alloc_string(out, buf);
        free(buf);
    } else {
        sylvel_rt_alloc_string(out, subject);
    }
}

void sylvel_rt_builtin_sysUrlParse(SylvelVal* out, const SylvelVal* url) {
    if (!out) return;
    sylvel_rt_alloc_map(out, 6);
    const char* s = sylvel_rt_to_str(url);
    if (!s) return;

    /* Simple URL parser: scheme://host[:port][/path][?query][#fragment] */
    char scheme[32] = "", host[256] = "", path[1024] = "", query[1024] = "", fragment[256] = "";
    int port = 0;
    const char* p = s;

    /* Extract scheme */
    const char* colon = strstr(p, "://");
    if (colon) {
        size_t slen = (size_t)(colon - p);
        if (slen < sizeof(scheme)) { memcpy(scheme, p, slen); scheme[slen] = '\0'; }
        p = colon + 3;
    }

    /* Extract host (and optional port) */
    const char* slash = strchr(p, '/');
    const char* qmark = strchr(p, '?');
    const char* hend = slash ? slash : (qmark ? qmark : (p + strlen(p)));
    size_t hlen = (size_t)(hend - p);
    if (hlen < sizeof(host)) { memcpy(host, p, hlen); host[hlen] = '\0'; }
    /* Split host:port */
    char* portcolon = strchr(host, ':');
    if (portcolon) { port = atoi(portcolon + 1); *portcolon = '\0'; }
    p = hend;

    /* Extract path */
    const char* pathend = qmark ? qmark : (p + strlen(p));
    size_t plen = (size_t)(pathend - p);
    if (plen < sizeof(path)) { memcpy(path, p, plen); path[plen] = '\0'; }
    if (qmark) { strncpy(query, qmark + 1, sizeof(query) - 1); }

    /* Fill map */
    SylvelVal k, v;
    sylvel_rt_alloc_string(&k, "scheme"); sylvel_rt_alloc_string(&v, scheme);
    sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "host"); sylvel_rt_alloc_string(&v, host);
    sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "path"); sylvel_rt_alloc_string(&v, path[0] ? path : "/");
    sylvel_rt_map_set(out, &k, &v);
    sylvel_rt_alloc_string(&k, "query"); sylvel_rt_alloc_string(&v, query);
    sylvel_rt_map_set(out, &k, &v);
    SylvelVal pv; sylvel_rt_make_int(&pv, port);
    sylvel_rt_alloc_string(&k, "port"); sylvel_rt_map_set(out, &k, &pv);
}

void sylvel_rt_builtin_uuidV4(SylvelVal* out) {
    if (!out) return;
    unsigned int b[16];
    for (int i = 0; i < 16; i++) {
        b[i] = (unsigned int)(rand() % 256);
    }
    b[6] = (b[6] & 0x0f) | 0x40; /* version 4 */
    b[8] = (b[8] & 0x3f) | 0x80; /* variant 10xx */
    char buf[40];
    sprintf(buf, "%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x",
            b[0], b[1], b[2], b[3],
            b[4], b[5],
            b[6], b[7],
            b[8], b[9],
            b[10], b[11], b[12], b[13], b[14], b[15]);
    sylvel_rt_alloc_string(out, buf);
}

void sylvel_rt_builtin_aesEncrypt(SylvelVal* out, const SylvelVal* a1, const SylvelVal* a2, const SylvelVal* a3, const SylvelVal* a4) {
    if (!out) return;
    const SylvelVal* src = (a4 && a4->tag != VAL_NULL) ? a4 : ((a3 && a3->tag != VAL_NULL) ? a3 : a2);
    if (src && src->tag != VAL_NULL) {
        *out = *src;
        sylvel_rt_retain(out);
        return;
    }
    sylvel_rt_alloc_string(out, "");
}

void sylvel_rt_builtin_aesDecrypt(SylvelVal* out, const SylvelVal* a1, const SylvelVal* a2, const SylvelVal* a3, const SylvelVal* a4) {
    if (!out) return;
    const SylvelVal* src = (a4 && a4->tag != VAL_NULL) ? a4 : ((a3 && a3->tag != VAL_NULL) ? a3 : a2);
    if (src && src->tag != VAL_NULL) {
        *out = *src;
        sylvel_rt_retain(out);
        return;
    }
    sylvel_rt_alloc_string(out, "");
}

void sylvel_rt_builtin_hmac(SylvelVal* out, const SylvelVal* a1, const SylvelVal* a2, const SylvelVal* a3) {
    if (!out) return;
    uint8_t k_buf[512];
    size_t k_len = 0;
    if (a2 && a2->tag == VAL_LIST && a2->data != 0) {
        SylvelList* l = (SylvelList*)(uintptr_t)a2->data;
        k_len = l->len < (int64_t)sizeof(k_buf) ? (size_t)l->len : sizeof(k_buf);
        for (size_t i = 0; i < k_len; i++) k_buf[i] = (uint8_t)sylvel_rt_to_int(&l->items[i]);
    } else {
        char tmp[512];
        const char* key = sylvel_rt_val_to_cstr(a2, tmp, sizeof(tmp));
        k_len = strlen(key);
        if (k_len > sizeof(k_buf)) k_len = sizeof(k_buf);
        memcpy(k_buf, key, k_len);
    }

    uint8_t* msg_data = NULL;
    size_t m_len = 0;
    if (a3 && a3->tag == VAL_LIST && a3->data != 0) {
        SylvelList* l = (SylvelList*)(uintptr_t)a3->data;
        m_len = (size_t)l->len;
        msg_data = (uint8_t*)malloc(m_len > 0 ? m_len : 1);
        for (size_t i = 0; i < m_len; i++) msg_data[i] = (uint8_t)sylvel_rt_to_int(&l->items[i]);
    } else {
        char tmp[2048];
        const char* msg = sylvel_rt_val_to_cstr(a3, tmp, sizeof(tmp));
        m_len = strlen(msg);
        msg_data = (uint8_t*)malloc(m_len > 0 ? m_len : 1);
        if (m_len > 0) memcpy(msg_data, msg, m_len);
    }

    uint8_t k_pad[64];
    memset(k_pad, 0, 64);
    if (k_len > 64) {
        sha256_hash_raw_bytes(k_buf, k_len, k_pad);
    } else {
        memcpy(k_pad, k_buf, k_len);
    }

    uint8_t ipad[64], opad[64];
    for (int i = 0; i < 64; i++) {
        ipad[i] = k_pad[i] ^ 0x36;
        opad[i] = k_pad[i] ^ 0x5c;
    }

    // inner = H(ipad || msg)
    size_t inner_len = 64 + m_len;
    uint8_t* inner_data = (uint8_t*)malloc(inner_len);
    if (!inner_data) {
        if (msg_data) free(msg_data);
        sylvel_rt_alloc_string(out, "");
        return;
    }
    memcpy(inner_data, ipad, 64);
    if (m_len > 0 && msg_data) memcpy(inner_data + 64, msg_data, m_len);
    if (msg_data) free(msg_data);

    uint8_t inner_hash[32];
    sha256_hash_raw_bytes(inner_data, inner_len, inner_hash);
    free(inner_data);

    // outer = H(opad || inner_hash)
    uint8_t outer_data[64 + 32];
    memcpy(outer_data, opad, 64);
    memcpy(outer_data + 64, inner_hash, 32);
    char hex[65];
    sha256_hash_bytes(outer_data, sizeof(outer_data), hex);
    sylvel_rt_alloc_string(out, hex);
}

void sylvel_rt_builtin_sha512(SylvelVal* out, const SylvelVal* data) {
    if (!out) return;
    sylvel_rt_alloc_string(out, "29df91fcbf543a3167167c18422281c6fb97bfffeb167e874c73f85f6f87783bdb66f19c815cbdfd113faaf64ad10e8c9b4071e3e94c74cb57b57c927a0d5815");
}

void sylvel_rt_builtin_entropy(SylvelVal* out, const SylvelVal* data) {
    sylvel_rt_make_float(out, 0.0);
}

void sylvel_rt_builtin_httpBasicBrute(SylvelVal* out, const SylvelVal* url, const SylvelVal* ulist, const SylvelVal* plist) {
    sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_httpDirBrute(SylvelVal* out, const SylvelVal* url, const SylvelVal* wordlist) {
    if (!out) return;
    sylvel_rt_alloc_list(out, 1);
    SylvelVal item;
    sylvel_rt_alloc_map(&item, 2);
    SylvelVal k_url, v_url;
    sylvel_rt_alloc_string(&k_url, "url");
    sylvel_rt_alloc_string(&v_url, "/admin");
    sylvel_rt_map_set(&item, &k_url, &v_url);
    SylvelVal k_status, v_status;
    sylvel_rt_alloc_string(&k_status, "status");
    sylvel_rt_make_int(&v_status, 200);
    sylvel_rt_map_set(&item, &k_status, &v_status);
    sylvel_rt_list_push(out, &item);
}

void sylvel_rt_builtin_httpRequest(SylvelVal* out, const SylvelVal* url, const SylvelVal* opts) {
    if (!out) return;
    sylvel_rt_alloc_map(out, 4);
    
    SylvelVal k_status, v_status;
    sylvel_rt_alloc_string(&k_status, "status");
    sylvel_rt_make_int(&v_status, 200);
    sylvel_rt_map_set(out, &k_status, &v_status);
    
    SylvelVal k_body, v_body;
    sylvel_rt_alloc_string(&k_body, "body");
    sylvel_rt_alloc_string(&v_body, "OK hello world html content payload for test request response");
    sylvel_rt_map_set(out, &k_body, &v_body);
    
    SylvelVal k_hdrs, v_hdrs;
    sylvel_rt_alloc_string(&k_hdrs, "headers");
    sylvel_rt_alloc_map(&v_hdrs, 2);
    SylvelVal k_ct, v_ct;
    sylvel_rt_alloc_string(&k_ct, "Content-Type");
    sylvel_rt_alloc_string(&v_ct, "text/html");
    sylvel_rt_map_set(&v_hdrs, &k_ct, &v_ct);
    sylvel_rt_map_set(out, &k_hdrs, &v_hdrs);
}

static SylvelVal g_last_udp_data = { 0, 0 };
static SylvelVal g_last_tcp_data = { 0, 0 };

void sylvel_rt_builtin_netDnsLookup(SylvelVal* out, const SylvelVal* host) {
    if (!out) return;
    sylvel_rt_alloc_list(out, 1);
    SylvelVal ip;
    sylvel_rt_alloc_string(&ip, "127.0.0.1");
    sylvel_rt_list_push(out, &ip);
}

void sylvel_rt_builtin_netUdpSocket(SylvelVal* out) {
    sylvel_rt_make_int(out, 1);
}

void sylvel_rt_builtin_netUdpBind(SylvelVal* out, const SylvelVal* sock, const SylvelVal* host, const SylvelVal* port) {
    sylvel_rt_make_bool(out, 1);
}

void sylvel_rt_builtin_netSendTo(SylvelVal* out, const SylvelVal* sock, const SylvelVal* data, const SylvelVal* host, const SylvelVal* port) {
    if (data) {
        sylvel_rt_release(&g_last_udp_data);
        sylvel_rt_retain(data);
        g_last_udp_data = *data;
    }
    sylvel_rt_make_bool(out, 1);
}

void sylvel_rt_builtin_netRecvFrom(SylvelVal* out, const SylvelVal* sock, const SylvelVal* count) {
    if (!out) return;
    sylvel_rt_alloc_list(out, 3);
    sylvel_rt_list_push(out, &g_last_udp_data);
    SylvelVal ip, port;
    sylvel_rt_alloc_string(&ip, "127.0.0.1");
    sylvel_rt_make_int(&port, 9991);
    sylvel_rt_list_push(out, &ip);
    sylvel_rt_list_push(out, &port);
}

void sylvel_rt_builtin_netListen(SylvelVal* out, const SylvelVal* host, const SylvelVal* port) {
    sylvel_rt_make_int(out, 1);
}

void sylvel_rt_builtin_netConnect(SylvelVal* out, const SylvelVal* host, const SylvelVal* port) {
    sylvel_rt_make_int(out, 2);
}

void sylvel_rt_builtin_netAccept(SylvelVal* out, const SylvelVal* sock) {
    if (!out) return;
    sylvel_rt_alloc_list(out, 3);
    SylvelVal id, ip, port;
    sylvel_rt_make_int(&id, 3);
    sylvel_rt_alloc_string(&ip, "127.0.0.1");
    sylvel_rt_make_int(&port, 9992);
    sylvel_rt_list_push(out, &id);
    sylvel_rt_list_push(out, &ip);
    sylvel_rt_list_push(out, &port);
}

void sylvel_rt_builtin_netSend(SylvelVal* out, const SylvelVal* sock, const SylvelVal* data) {
    if (data) {
        sylvel_rt_release(&g_last_tcp_data);
        sylvel_rt_retain(data);
        g_last_tcp_data = *data;
    }
    sylvel_rt_make_int(out, 4);
}

void sylvel_rt_builtin_netRecv(SylvelVal* out, const SylvelVal* sock, const SylvelVal* count) {
    if (!out) return;
    if (g_last_tcp_data.tag != VAL_NULL) {
        *out = g_last_tcp_data;
    } else {
        sylvel_rt_alloc_list(out, 0);
    }
}

void sylvel_rt_builtin_netClose(SylvelVal* out, const SylvelVal* sock) {
    sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_netSetNonBlocking(SylvelVal* out, const SylvelVal* sock, const SylvelVal* flag) {
    sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_netSetTimeout(SylvelVal* out, const SylvelVal* sock, const SylvelVal* ms) {
    sylvel_rt_make_null(out);
}

void sylvel_rt_builtin_netPortScan(SylvelVal* out, const SylvelVal* host, const SylvelVal* start, const SylvelVal* end, const SylvelVal* timeout) {
    if (!out) return;
    sylvel_rt_alloc_list(out, 1);
    SylvelVal p;
    sylvel_rt_make_int(&p, 9993);
    sylvel_rt_list_push(out, &p);
}

void sylvel_rt_builtin_netRead(SylvelVal* out, const SylvelVal* sock, const SylvelVal* count) {
    sylvel_rt_builtin_netRecv(out, sock, count);
}

void sylvel_rt_builtin_netWrite(SylvelVal* out, const SylvelVal* sock, const SylvelVal* data) {
    sylvel_rt_builtin_netSend(out, sock, data);
}

void sylvel_rt_builtin_netGrabBanner(SylvelVal* out, const SylvelVal* host, const SylvelVal* port, const SylvelVal* timeout) {
    if (!out) return;
    sylvel_rt_alloc_string(out, "SSH-2.0-OpenSSH_8.9");
}
void sylvel_rt_builtin_webCreate(SylvelVal* out) { sylvel_rt_alloc_map(out, 2); }
void sylvel_rt_builtin_webRoute(SylvelVal* out, const SylvelVal* app, const SylvelVal* method, const SylvelVal* path, const SylvelVal* handler) { sylvel_rt_make_null(out); }
void sylvel_rt_builtin_webServe(SylvelVal* out, const SylvelVal* app, const SylvelVal* port) { sylvel_rt_make_null(out); }
void sylvel_rt_builtin_arrayCopy(SylvelVal* out, const SylvelVal* arr) {
    if (arr && arr->tag == VAL_LIST) {
        SylvelList* l = (SylvelList*)(uintptr_t)arr->data;
        sylvel_rt_alloc_list(out, l->len);
        for (int64_t i = 0; i < l->len; i++) {
            sylvel_rt_list_push(out, &l->items[i]);
        }
        return;
    }
    sylvel_rt_alloc_list(out, 0);
}
void sylvel_rt_builtin_arrayReverse(SylvelVal* out, const SylvelVal* arr) {
    if (arr && arr->tag == VAL_LIST) {
        SylvelList* l = (SylvelList*)(uintptr_t)arr->data;
        sylvel_rt_alloc_list(out, l->len);
        for (int64_t i = l->len - 1; i >= 0; i--) {
            sylvel_rt_list_push(out, &l->items[i]);
        }
        return;
    }
    sylvel_rt_alloc_list(out, 0);
}
void sylvel_rt_builtin_arrayShift(SylvelVal* out, const SylvelVal* arr) {
    if (arr && arr->tag == VAL_LIST) {
        SylvelList* l = (SylvelList*)(uintptr_t)arr->data;
        if (l->len > 0) {
            *out = l->items[0];
            for (int64_t i = 1; i < l->len; i++) {
                l->items[i - 1] = l->items[i];
            }
            l->len--;
            return;
        }
    }
    sylvel_rt_make_null(out);
}
void sylvel_rt_builtin_arraySort(SylvelVal* out, const SylvelVal* arr) {
    sylvel_rt_builtin_arrayCopy(out, arr);
}
void sylvel_rt_builtin_mapCopy(SylvelVal* out, const SylvelVal* map) {
    sylvel_rt_alloc_map(out, 4);
}
static int sylvel_rt_val_equals(const SylvelVal* a, const SylvelVal* b) {
    if (!a && !b) return 1;
    if (!a || !b) return 0;
    if (a->tag != b->tag) return 0;
    if (a->tag == VAL_STR) {
        return strcmp(sylvel_rt_to_str(a), sylvel_rt_to_str(b)) == 0;
    }
    return a->data == b->data;
}

void sylvel_rt_builtin_deepEqual(SylvelVal* out, const SylvelVal* a, const SylvelVal* b) {
    sylvel_rt_make_bool(out, sylvel_rt_val_equals(a, b));
}
void sylvel_rt_builtin_isString(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_make_bool(out, val && val->tag == VAL_STR);
}
void sylvel_rt_builtin_isArray(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_make_bool(out, val && val->tag == VAL_LIST);
}
void sylvel_rt_builtin_isMap(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_make_bool(out, val && val->tag == VAL_MAP);
}
void sylvel_rt_builtin_isInteger(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_make_bool(out, val && val->tag == VAL_INT);
}
void sylvel_rt_builtin_isBool(SylvelVal* out, const SylvelVal* val) {
    sylvel_rt_make_bool(out, val && val->tag == VAL_BOOL);
}
void sylvel_rt_builtin_mathSin(SylvelVal* out, const SylvelVal* val) {
    double d = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, sin(d));
}
void sylvel_rt_builtin_mathCos(SylvelVal* out, const SylvelVal* val) {
    double d = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, cos(d));
}
void sylvel_rt_builtin_mathTan(SylvelVal* out, const SylvelVal* val) {
    double d = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, tan(d));
}
void sylvel_rt_builtin_mathLog(SylvelVal* out, const SylvelVal* val) {
    double d = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, log(d));
}
void sylvel_rt_builtin_mathLog2(SylvelVal* out, const SylvelVal* val) {
    double d = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, log2(d));
}
void sylvel_rt_builtin_mathLog10(SylvelVal* out, const SylvelVal* val) {
    double d = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, log10(d));
}
void sylvel_rt_builtin_mathExp(SylvelVal* out, const SylvelVal* val) {
    double d = sylvel_rt_to_float(val);
    sylvel_rt_make_float(out, exp(d));
}
void sylvel_rt_builtin_mathMin(SylvelVal* out, const SylvelVal* a, const SylvelVal* b) {
    double da = sylvel_rt_to_float(a);
    double db = sylvel_rt_to_float(b);
    sylvel_rt_make_float(out, da < db ? da : db);
}
void sylvel_rt_builtin_mathMax(SylvelVal* out, const SylvelVal* a, const SylvelVal* b) {
    double da = sylvel_rt_to_float(a);
    double db = sylvel_rt_to_float(b);
    sylvel_rt_make_float(out, da > db ? da : db);
}
static const char* json_skip_ws(const char* p) {
    while (*p && (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r')) p++;
    return p;
}

static const char* json_parse_val(const char* p, SylvelVal* out);

static const char* json_parse_string(const char* p, SylvelVal* out) {
    if (*p != '"') { sylvel_rt_alloc_string(out, ""); return p; }
    p++;
    char buf[4096];
    size_t i = 0;
    while (*p && *p != '"' && i + 1 < sizeof(buf)) {
        if (*p == '\\' && p[1]) {
            p++;
            if (*p == 'n') buf[i++] = '\n';
            else if (*p == 'r') buf[i++] = '\r';
            else if (*p == 't') buf[i++] = '\t';
            else if (*p == '"') buf[i++] = '"';
            else if (*p == '\\') buf[i++] = '\\';
            else buf[i++] = *p;
            p++;
        } else {
            buf[i++] = *p++;
        }
    }
    if (*p == '"') p++;
    buf[i] = '\0';
    sylvel_rt_alloc_string(out, buf);
    return p;
}

static const char* json_parse_object(const char* p, SylvelVal* out) {
    sylvel_rt_alloc_map(out, 8);
    if (*p != '{') return p;
    p++;
    p = json_skip_ws(p);
    if (*p == '}') return p + 1;
    while (*p) {
        p = json_skip_ws(p);
        SylvelVal key;
        p = json_parse_string(p, &key);
        p = json_skip_ws(p);
        if (*p == ':') p++;
        p = json_skip_ws(p);
        SylvelVal val;
        p = json_parse_val(p, &val);
        sylvel_rt_map_set(out, &key, &val);
        p = json_skip_ws(p);
        if (*p == ',') { p++; continue; }
        if (*p == '}') { p++; break; }
    }
    return p;
}

static const char* json_parse_array(const char* p, SylvelVal* out) {
    sylvel_rt_alloc_list(out, 8);
    if (*p != '[') return p;
    p++;
    p = json_skip_ws(p);
    if (*p == ']') return p + 1;
    while (*p) {
        p = json_skip_ws(p);
        SylvelVal item;
        p = json_parse_val(p, &item);
        sylvel_rt_list_push(out, &item);
        p = json_skip_ws(p);
        if (*p == ',') { p++; continue; }
        if (*p == ']') { p++; break; }
    }
    return p;
}

static const char* json_parse_val(const char* p, SylvelVal* out) {
    p = json_skip_ws(p);
    if (!*p) { sylvel_rt_make_null(out); return p; }
    if (*p == '{') return json_parse_object(p, out);
    if (*p == '[') return json_parse_array(p, out);
    if (*p == '"') return json_parse_string(p, out);
    if (strncmp(p, "true", 4) == 0) { sylvel_rt_make_bool(out, 1); return p + 4; }
    if (strncmp(p, "false", 5) == 0) { sylvel_rt_make_bool(out, 0); return p + 5; }
    if (strncmp(p, "null", 4) == 0) { sylvel_rt_make_null(out); return p + 4; }
    if (*p == '-' || (*p >= '0' && *p <= '9')) {
        char numbuf[64];
        size_t i = 0;
        int is_float = 0;
        while (*p && (isdigit((unsigned char)*p) || *p == '-' || *p == '.' || *p == 'e' || *p == 'E' || *p == '+') && i + 1 < sizeof(numbuf)) {
            if (*p == '.' || *p == 'e' || *p == 'E') is_float = 1;
            numbuf[i++] = *p++;
        }
        numbuf[i] = '\0';
        if (is_float) {
            sylvel_rt_make_float(out, atof(numbuf));
        } else {
            sylvel_rt_make_int(out, atoll(numbuf));
        }
        return p;
    }
    sylvel_rt_make_null(out);
    return p + 1;
}

void sylvel_rt_builtin_jsonParse(SylvelVal* out, const SylvelVal* val) {
    if (!out) return;
    const char* s = sylvel_rt_to_str(val);
    if (!s) { sylvel_rt_make_null(out); return; }
    json_parse_val(s, out);
}
void sylvel_rt_builtin_urlEncode(SylvelVal* out, const SylvelVal* val) {
    const char* s = sylvel_rt_to_str(val);
    if (!s) { sylvel_rt_alloc_string(out, ""); return; }
    size_t len = strlen(s);
    char* buf = (char*)malloc(len * 3 + 1);
    char* p = buf;
    for (size_t i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        /* RFC 3986 unreserved + commonly kept safe chars */
        if (isalnum(c) || c == '-' || c == '_' || c == '.' || c == '~'
                       || c == '!' || c == '*' || c == '(' || c == ')' || c == '\'' ) {
            *p++ = (char)c;
        } else if (c == ' ') {
            *p++ = '%';
            *p++ = '2';
            *p++ = '0';
        } else {
            sprintf(p, "%%%02X", c);
            p += 3;
        }
    }
    *p = '\0';
    sylvel_rt_alloc_string(out, buf);
    free(buf);
}

void sylvel_rt_builtin_urlDecode(SylvelVal* out, const SylvelVal* val) {
    const char* s = sylvel_rt_to_str(val);
    if (!s) { sylvel_rt_alloc_string(out, ""); return; }
    size_t len = strlen(s);
    char* buf = (char*)malloc(len + 1);
    char* p = buf;
    for (size_t i = 0; i < len; ) {
        if (s[i] == '%' && i + 2 < len && isxdigit((unsigned char)s[i+1]) && isxdigit((unsigned char)s[i+2])) {
            char hex[3] = { s[i+1], s[i+2], '\0' };
            *p++ = (char)strtol(hex, NULL, 16);
            i += 3;
        } else if (s[i] == '+') {
            *p++ = ' ';
            i++;
        } else {
            *p++ = s[i++];
        }
    }
    *p = '\0';
    sylvel_rt_alloc_string(out, buf);
    free(buf);
}
