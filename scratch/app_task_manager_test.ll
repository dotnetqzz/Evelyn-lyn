; ModuleID = 'avelyn_module'
target datalayout = "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-windows-msvc"

%SylvelVal = type { i32, i32, i64 }

declare void @sylvel_rt_make_null(%SylvelVal*)
declare void @sylvel_rt_make_bool(%SylvelVal*, i32)
declare void @sylvel_rt_make_int(%SylvelVal*, i64)
declare void @sylvel_rt_make_float(%SylvelVal*, double)
declare void @sylvel_rt_alloc_string(%SylvelVal*, i8*)
declare void @sylvel_rt_alloc_list(%SylvelVal*, i64)
declare void @sylvel_rt_alloc_map(%SylvelVal*, i64)
declare void @sylvel_rt_print(%SylvelVal*)
declare void @sylvel_rt_bin_op(%SylvelVal*, %SylvelVal*, i32, %SylvelVal*)
declare void @sylvel_rt_unary_op(%SylvelVal*, i32, %SylvelVal*)
declare void @sylvel_rt_list_push(%SylvelVal*, %SylvelVal*)
declare void @sylvel_rt_list_get(%SylvelVal*, %SylvelVal*, i64)
declare void @sylvel_rt_list_set(%SylvelVal*, i64, %SylvelVal*)
declare void @sylvel_rt_map_get(%SylvelVal*, %SylvelVal*, %SylvelVal*)
declare void @sylvel_rt_map_set(%SylvelVal*, %SylvelVal*, %SylvelVal*)
declare void @sylvel_rt_subscript_get(%SylvelVal*, %SylvelVal*, %SylvelVal*)
declare void @sylvel_rt_subscript_set(%SylvelVal*, %SylvelVal*, %SylvelVal*)
declare void @sylvel_rt_call_expr(%SylvelVal*, %SylvelVal*, %SylvelVal*, %SylvelVal*)
declare void @sylvel_rt_builtin_assert(%SylvelVal*, %SylvelVal*, %SylvelVal*)
declare void @sylvel_rt_enter_try()
declare void @sylvel_rt_exit_try()
declare i32 @sylvel_rt_has_error()
declare void @sylvel_rt_clear_error()
declare void @sylvel_rt_raise_error(i8*)
declare i64 @sylvel_rt_len(%SylvelVal*)
declare i1 @sylvel_rt_to_bool(%SylvelVal*)
declare i64 @sylvel_rt_to_int(%SylvelVal*)
declare double @sylvel_rt_to_float(%SylvelVal*)
declare void @sylvel_rt_retain(%SylvelVal*)
declare void @sylvel_rt_release(%SylvelVal*)



define i32 @main() {
entry:
  %t1 = alloca %SylvelVal
  %t2 = alloca %SylvelVal
  %t3 = alloca %SylvelVal
  %t4 = alloca %SylvelVal
  %t5 = alloca %SylvelVal
  %t6 = alloca %SylvelVal
  %t7 = alloca %SylvelVal
  %t8 = alloca %SylvelVal
  %t9 = alloca %SylvelVal
  %t10 = alloca %SylvelVal
  %t11 = alloca %SylvelVal
  %t12 = alloca %SylvelVal
  %t13 = alloca %SylvelVal
  %t14 = alloca %SylvelVal
  %t15 = alloca %SylvelVal
  %t16 = alloca %SylvelVal
  %t17 = alloca %SylvelVal
  %t18 = alloca %SylvelVal
  %t19 = alloca %SylvelVal
  %t20 = alloca %SylvelVal
  %t21 = alloca %SylvelVal
  %t22 = alloca %SylvelVal
  %t23 = alloca %SylvelVal
  %t24 = alloca %SylvelVal
  %t25 = alloca %SylvelVal
  %t26 = alloca %SylvelVal
  %t27 = alloca %SylvelVal
  call void @sylvel_rt_make_null(%SylvelVal* %t1)
  call void @sylvel_rt_make_null(%SylvelVal* %t2)
  call void @sylvel_rt_make_null(%SylvelVal* %t3)
  call void @sylvel_rt_make_null(%SylvelVal* %t4)
  call void @sylvel_rt_make_null(%SylvelVal* %t5)
  call void @sylvel_rt_make_null(%SylvelVal* %t6)
  call void @sylvel_rt_make_null(%SylvelVal* %t7)
  call void @sylvel_rt_make_null(%SylvelVal* %t8)
  call void @sylvel_rt_make_null(%SylvelVal* %t9)
  call void @sylvel_rt_make_null(%SylvelVal* %t10)
  call void @sylvel_rt_make_null(%SylvelVal* %t11)
  call void @sylvel_rt_make_null(%SylvelVal* %t12)
  call void @sylvel_rt_make_null(%SylvelVal* %t13)
  call void @sylvel_rt_make_null(%SylvelVal* %t14)
  call void @sylvel_rt_make_null(%SylvelVal* %t15)
  call void @sylvel_rt_make_null(%SylvelVal* %t16)
  call void @sylvel_rt_make_null(%SylvelVal* %t17)
  call void @sylvel_rt_make_null(%SylvelVal* %t18)
  call void @sylvel_rt_make_null(%SylvelVal* %t19)
  call void @sylvel_rt_make_null(%SylvelVal* %t20)
  call void @sylvel_rt_make_null(%SylvelVal* %t21)
  call void @sylvel_rt_make_null(%SylvelVal* %t22)
  call void @sylvel_rt_make_null(%SylvelVal* %t23)
  call void @sylvel_rt_make_null(%SylvelVal* %t24)
  call void @sylvel_rt_make_null(%SylvelVal* %t25)
  call void @sylvel_rt_make_null(%SylvelVal* %t26)
  call void @sylvel_rt_make_null(%SylvelVal* %t27)
  ret i32 0
}
