
bool FUN_18005e770(longlong *param_1)

{
  uint uVar1;
  longlong lVar2;
  longlong *plVar3;
  short local_18 [2];
  undefined4 local_14;
  
  local_18[0] = (short)*(char *)((longlong)param_1 + 0x1c);
  local_18[1] = 3;
  local_14 = (undefined4)param_1[2];
  lVar2 = (**(code **)(*param_1 + 8))(param_1,local_18);
  if ((lVar2 != 0) && (uVar1 = *(uint *)(lVar2 + 0xf0), uVar1 != 0xffffffff)) {
    plVar3 = (longlong *)FUN_1807d08e0();
    if ((ulonglong)uVar1 < (ulonglong)(plVar3[1] - *plVar3 >> 3)) {
      return *(int *)(*(longlong *)(*plVar3 + (ulonglong)uVar1 * 8) + 0x1c) == -0x75cdff83;
    }
    return iRam000000000000001c == -0x75cdff83;
  }
  return false;
}


