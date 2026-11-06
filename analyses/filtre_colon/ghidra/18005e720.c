
undefined4 FUN_18005e720(longlong *param_1)

{
  longlong lVar1;
  short local_18 [2];
  undefined4 local_14;
  
  local_18[0] = (short)*(char *)((longlong)param_1 + 0x1c);
  local_18[1] = 3;
  local_14 = (undefined4)param_1[2];
  lVar1 = (**(code **)(*param_1 + 8))(param_1,local_18);
  if (lVar1 != 0) {
    return *(undefined4 *)(lVar1 + 0xf0);
  }
  return 0xffffffff;
}


