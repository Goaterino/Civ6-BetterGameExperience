
undefined8 FUN_180017b30(lua_State *param_1)

{
  int iVar1;
  char cVar2;
  int iVar3;
  int iVar4;
  uint uVar5;
  int iVar6;
  longlong lVar7;
  undefined8 uVar8;
  longlong *plVar9;
  longlong lVar10;
  HksObject *pHVar11;
  int iVar12;
  HksObject *pHVar13;
  int iVar14;
  int iVar15;
  int iVar16;
  int iStack_34;
  int iStack_2c;
  
  lVar7 = FUN_180044d40();
  iVar1 = *(int *)(lVar7 + 0x30);
  iVar3 = FUN_180044e60();
  uVar8 = FUN_180044d40();
  lVar7 = FUN_180052cf0(uVar8,iVar3);
  lua_createtable(param_1,iVar1,0);
  lua_createtable(param_1,iVar1,0);
  lua_createtable(param_1,iVar1,0);
  lua_createtable(param_1,iVar1,0);
  if ((-1 < iVar3) && (iVar3 = 0, 0 < iVar1)) {
    iStack_2c = 1;
    iVar4 = 1;
    iStack_34 = 1;
    iVar14 = iVar4;
    do {
      plVar9 = (longlong *)FUN_180044d40();
      lVar10 = (**(code **)(*plVar9 + 0x60))(plVar9,iVar3);
      uVar5 = *(int *)(lVar7 + 200) * (int)*(short *)(lVar10 + 0x2a) +
              (int)*(short *)(lVar10 + 0x28);
      if ((((uVar5 < *(uint *)(*(longlong *)(lVar7 + 0x120) + 8)) &&
           (0 < *(short *)(*(longlong *)(*(longlong *)(lVar7 + 0x120) + 0x10) + (ulonglong)uVar5 * 2
                          ))) &&
          (*(char *)((ulonglong)uVar5 + *(longlong *)(lVar7 + 0x138)) != '\0')) &&
         (-1 < *(int *)(lVar10 + 0x30))) {
        iVar6 = FUN_180144810(0xffffffff);
        iVar12 = iStack_2c;
        iVar15 = iVar14;
        iVar16 = iStack_34;
        if (iVar6 == 1) {
          cVar2 = FUN_180060010(lVar10);
          iVar6 = iVar4;
          if (cVar2 == '\0') {
            cVar2 = FUN_18005fe00(lVar10,0xffffffff);
            pHVar13 = *(HksObject **)(param_1 + 0x48);
            *(undefined4 *)pHVar13 = 3;
            *(double *)(pHVar13 + 8) = (double)iVar3;
            *(HksObject **)(param_1 + 0x48) = pHVar13 + 0x10;
            if (cVar2 == '\0') {
              iVar12 = iStack_2c + 1;
              pHVar11 = pHVar13 + -0x20;
              iVar14 = iStack_2c;
            }
            else {
              pHVar11 = pHVar13 + -0x30;
              iVar15 = iVar14 + 1;
            }
          }
          else {
            pHVar13 = *(HksObject **)(param_1 + 0x48);
            *(undefined4 *)pHVar13 = 3;
            *(double *)(pHVar13 + 8) = (double)iVar3;
            *(HksObject **)(param_1 + 0x48) = pHVar13 + 0x10;
            pHVar11 = pHVar13 + -0x40;
            iVar16 = iStack_34 + 1;
            iVar14 = iStack_34;
          }
        }
        else {
          pHVar13 = *(HksObject **)(param_1 + 0x48);
          *(undefined4 *)pHVar13 = 3;
          *(double *)(pHVar13 + 8) = (double)iVar3;
          *(HksObject **)(param_1 + 0x48) = pHVar13 + 0x10;
          pHVar11 = pHVar13 + -0x10;
          iVar6 = iVar4 + 1;
          iVar14 = iVar4;
        }
        hks_obj_rawseti(param_1,pHVar11,iVar14,pHVar13);
        *(longlong *)(param_1 + 0x48) = *(longlong *)(param_1 + 0x48) + -0x10;
        iStack_2c = iVar12;
        iVar4 = iVar6;
        iVar14 = iVar15;
        iStack_34 = iVar16;
      }
      iVar3 = iVar3 + 1;
    } while (iVar3 < iVar1);
  }
  return 4;
}


