
void FUN_140199b30(longlong param_1,int param_2)

{
  longlong lVar1;
  longlong lVar2;
  bool bVar3;
  uint uVar4;
  int iVar5;
  undefined4 *puVar6;
  uint uVar7;
  uint uVar8;
  ulonglong uVar9;
  
  if (*(char *)(param_1 + 0x1d8) != '\0') {
    uVar7 = *(uint *)(param_1 + 0x48);
    lVar1 = *(longlong *)(param_1 + 0x58);
    uVar4 = FUN_14004f970(uVar7,lVar1,param_2,*(undefined4 *)(param_1 + 0x50));
    uVar9 = (ulonglong)uVar4;
    uVar8 = 0xffffffff;
    if (uVar4 != 0xffffffff) {
      iVar5 = *(int *)(lVar1 + uVar9 * 4);
      while (uVar8 = 0xffffffff, iVar5 != -1) {
        if (*(int *)(lVar1 + uVar9 * 4) == param_2) {
          uVar8 = *(uint *)(*(longlong *)(param_1 + 0x60) + uVar9 * 4);
          break;
        }
        uVar9 = (ulonglong)((int)uVar9 + 1) % (ulonglong)uVar7;
        iVar5 = *(int *)(lVar1 + uVar9 * 4);
      }
    }
    if (uVar8 < *(uint *)(param_1 + 0x40)) {
      lVar1 = (ulonglong)(uVar8 >> 5) * 4;
      uVar7 = 1 << (sbyte)(uVar8 & 0x1f);
      lVar2 = *(longlong *)(param_1 + 400);
      uVar4 = *(uint *)(*(longlong *)(*(longlong *)(param_1 + 0x1d0) + 0x138) + lVar1) & uVar7;
      if ((((*(uint *)(lVar1 + lVar2) & uVar7) == 0) &&
          ((*(uint *)(lVar1 + *(longlong *)(*(longlong *)(param_1 + 0x1d0) + 0x118)) & uVar7) == 0))
         || (uVar4 == 0)) {
        bVar3 = false;
      }
      else {
        bVar3 = true;
      }
      *(uint *)(lVar1 + lVar2) = *(uint *)(lVar1 + lVar2) | 1 << (uVar8 & 0x1f);
      iVar5 = FUN_140093280("orldCursor",10,0xd20c0cd6);
      if ((((!bVar3) && (uVar4 != 0)) || (param_2 == iVar5)) &&
         (puVar6 = (undefined4 *)FUN_140196150(&DAT_141b0b380 + (ulonglong)DAT_141b0b304 * 0x200060)
         , puVar6 != (undefined4 *)0x0)) {
        *puVar6 = *(undefined4 *)((ulonglong)uVar8 * 0x20 + *(longlong *)(param_1 + 0x30));
      }
    }
  }
  return;
}


