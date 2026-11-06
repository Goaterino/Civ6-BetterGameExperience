
/* WARNING: Removing unreachable block (ram,0x00014019c506) */
/* WARNING: Removing unreachable block (ram,0x00014019c3ae) */
/* WARNING: Removing unreachable block (ram,0x00014019c33e) */
/* WARNING: Removing unreachable block (ram,0x00014019c3e0) */
/* WARNING: Removing unreachable block (ram,0x00014019c527) */

undefined8 FUN_14019c310(lua_State *param_1)

{
  int iVar1;
  undefined4 uVar2;
  int iVar3;
  char *pcVar4;
  HksObject *pHVar5;
  HksObject *pHVar6;
  HksObject *pHVar7;
  HksObject *pHVar8;
  HksObject *pHVar9;
  HksObject *pHVar10;
  double dVar11;
  int iVar12;
  void *local_48;
  undefined4 local_40;
  undefined8 local_3c;
  undefined4 local_34;
  void *local_30;
  undefined4 local_28;
  undefined8 local_24;
  undefined4 local_1c;
  
  luaL_where(param_1,1);
  pHVar8 = (HksObject *)0x0;
  iVar12 = 0;
  if (*(HksObject **)(param_1 + 0x50) <= (HksObject *)(*(longlong *)(param_1 + 0x48) + -0x10)) {
    hks_obj_tolstring(param_1,(HksObject *)(*(longlong *)(param_1 + 0x48) + -0x10),(__uint64 *)0x0);
  }
  *(longlong *)(param_1 + 0x48) = *(longlong *)(param_1 + 0x48) + -0x10;
  pHVar6 = *(HksObject **)(param_1 + 0x50);
  pHVar9 = *(HksObject **)(param_1 + 0x48);
  if ((pHVar9 <= pHVar6) || (iVar1 = hks_obj_isnumber(param_1,pHVar6), iVar1 == 0)) {
    return 0;
  }
  uVar2 = FUN_14019a3e0(param_1,1);
  if (*(HksObject **)(param_1 + 0x48) <= (HksObject *)(*(longlong *)(param_1 + 0x50) + 0x10)) {
    return 0;
  }
  iVar1 = hks_obj_isnumber(param_1,(HksObject *)(*(longlong *)(param_1 + 0x50) + 0x10));
  if (iVar1 == 0) {
    return 0;
  }
  iVar1 = 0;
  pHVar10 = pHVar8;
  if ((HksObject *)(*(longlong *)(param_1 + 0x50) + 0x10) < *(HksObject **)(param_1 + 0x48)) {
    dVar11 = hks_obj_tonumber(param_1,(HksObject *)(*(longlong *)(param_1 + 0x50) + 0x10));
    pHVar10 = (HksObject *)(ulonglong)(uint)(int)dVar11;
  }
  pHVar5 = (HksObject *)(*(longlong *)(param_1 + 0x50) + 0x20);
  local_24 = 0;
  local_1c = 0;
  pHVar7 = pHVar5;
  if (*(HksObject **)(param_1 + 0x48) <= pHVar5) {
    pHVar7 = pHVar8;
  }
  local_30 = (void *)0x0;
  local_28 = 0;
  if (*(HksObject **)(param_1 + 0x48) <= pHVar5) goto LAB_14019c57f;
  if (((*(uint *)pHVar7 & 0xf) == 5) || (iVar3 = iVar1, (*(uint *)pHVar7 & 0xf) == 0xc)) {
    iVar3 = 1;
  }
  if (iVar3 == 0) goto LAB_14019c57f;
  FUN_14019a0c0(param_1,3,&local_30);
  if ((int)((longlong)pHVar9 - (longlong)pHVar6 >> 4) == 5) {
    pHVar6 = (HksObject *)(*(longlong *)(param_1 + 0x50) + 0x40);
    if ((pHVar6 < *(HksObject **)(param_1 + 0x48)) &&
       ((*(uint *)pHVar6 & 0xf) == 3 || (*(uint *)pHVar6 & 0xf) == 4)) {
      if (*(HksObject **)(param_1 + 0x48) <= pHVar6) {
        pHVar6 = pHVar8;
      }
      pcVar4 = hks_obj_tolstring(param_1,pHVar6,(__uint64 *)0x0);
      iVar1 = FUN_1409bba50(pcVar4);
    }
  }
  pHVar6 = *(HksObject **)(param_1 + 0x48);
  pHVar7 = (HksObject *)(*(longlong *)(param_1 + 0x50) + 0x30);
  local_3c = 0;
  local_34 = 0;
  pHVar9 = pHVar7;
  if (pHVar6 <= pHVar7) {
    pHVar9 = pHVar8;
  }
  local_48 = (void *)0x0;
  local_40 = 0;
  if (pHVar6 > pHVar7) {
    iVar3 = 1;
    if (((*(uint *)pHVar9 & 0xf) != 5) && ((*(uint *)pHVar9 & 0xf) != 0xc)) {
      iVar3 = iVar12;
    }
    if (iVar3 == 0) {
      if ((pHVar6 <= pHVar7) || (iVar3 = hks_obj_isnumber(param_1,pHVar7), iVar3 == 0))
      goto LAB_14019c568;
      if ((HksObject *)(*(longlong *)(param_1 + 0x50) + 0x30) < *(HksObject **)(param_1 + 0x48)) {
        dVar11 = hks_obj_tonumber(param_1,(HksObject *)(*(longlong *)(param_1 + 0x50) + 0x30));
        iVar12 = (int)dVar11;
      }
    }
    else {
      FUN_140199fc0(param_1,4,&local_48);
      iVar12 = -1;
    }
    FUN_140196c50(DAT_14230f760 + 0x450,uVar2,pHVar10,&local_30,&local_48,iVar1,iVar12);
  }
LAB_14019c568:
  _aligned_free(local_48);
LAB_14019c57f:
  _aligned_free(local_30);
  return 0;
}


