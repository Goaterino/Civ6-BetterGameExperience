
undefined8 FUN_1806a2070(lua_State *param_1)

{
  int iVar1;
  char cVar2;
  int iVar3;
  int iVar4;
  longlong lVar5;
  undefined8 uVar6;
  longlong lVar7;
  ulonglong uVar8;
  HksObject *pHVar9;
  longlong *plVar10;
  int iVar11;
  HksObject *pHVar12;
  int iVar13;
  longlong lVar14;
  int local_res10;
  int local_res18;
  int local_res20;
  int iStack_3c;
  
  FUN_1807d06e0();
  lVar5 = FUN_18007af10();
  iVar1 = *(int *)(lVar5 + 0x30);
  iVar3 = FUN_18007aef0();
  if (-1 < iVar3) {
    uVar6 = FUN_18007af10();
    lVar5 = FUN_18000a2a0(uVar6,iVar3);
    lua_createtable(param_1,iVar1,0);
    lua_createtable(param_1,iVar1,0);
    lua_createtable(param_1,iVar1,0);
    lua_createtable(param_1,iVar1,0);
    iVar11 = 0;
    if (0 < iVar1) {
      iStack_3c = 1;
      lVar14 = 0;
      local_res20 = 1;
      local_res18 = 1;
      local_res10 = 1;
      do {
        lVar7 = FUN_18007af10();
        if ((iVar11 < 0) || (*(int *)(lVar7 + 0x30) <= iVar11)) {
          plVar10 = (longlong *)0x0;
        }
        else {
          plVar10 = (longlong *)(*(longlong *)(lVar7 + 0x170) + lVar14);
        }
        uVar8 = (ulonglong)
                (uint)((int)(short)plVar10[5] +
                      *(int *)(lVar5 + 8) * (int)*(short *)((longlong)plVar10 + 0x2a));
        if (((0 < *(short *)(*(longlong *)(*(longlong *)(lVar5 + 0x10) + 0x10) + uVar8 * 2)) ||
            (*(char *)(uVar8 + *(longlong *)(lVar5 + 0x18)) != '\0')) && (-1 < (int)plVar10[6])) {
          iVar13 = iStack_3c;
          if (((*(byte *)(plVar10 + 10) & 1) == 0) &&
             ((lVar7 = (**(code **)(*plVar10 + 0x30))(plVar10), *(char *)(lVar7 + 0x1c) != iVar3 ||
              (iVar4 = FUN_180144810(iVar3,(int)(short)plVar10[5],
                                     (int)*(short *)((longlong)plVar10 + 0x2a)), iVar4 != 1)))) {
            pHVar12 = *(HksObject **)(param_1 + 0x48);
            *(undefined4 *)pHVar12 = 3;
            *(double *)(pHVar12 + 8) = (double)iVar11;
            *(HksObject **)(param_1 + 0x48) = pHVar12 + 0x10;
            pHVar9 = pHVar12 + -0x10;
            iStack_3c = local_res10;
            local_res10 = local_res10 + 1;
          }
          else {
            cVar2 = FUN_180060010(plVar10);
            if (cVar2 == '\0') {
              cVar2 = FUN_18005fe00(plVar10,0xffffffff);
              pHVar12 = *(HksObject **)(param_1 + 0x48);
              *(undefined4 *)pHVar12 = 3;
              *(double *)(pHVar12 + 8) = (double)iVar11;
              *(HksObject **)(param_1 + 0x48) = pHVar12 + 0x10;
              if (cVar2 == '\0') {
                iVar13 = iStack_3c + 1;
                pHVar9 = pHVar12 + -0x20;
              }
              else {
                pHVar9 = pHVar12 + -0x30;
                iStack_3c = local_res20;
                local_res20 = local_res20 + 1;
              }
            }
            else {
              pHVar12 = *(HksObject **)(param_1 + 0x48);
              *(undefined4 *)pHVar12 = 3;
              *(double *)(pHVar12 + 8) = (double)iVar11;
              *(HksObject **)(param_1 + 0x48) = pHVar12 + 0x10;
              pHVar9 = pHVar12 + -0x40;
              iStack_3c = local_res18;
              local_res18 = local_res18 + 1;
            }
          }
          hks_obj_rawseti(param_1,pHVar9,iStack_3c,pHVar12);
          *(longlong *)(param_1 + 0x48) = *(longlong *)(param_1 + 0x48) + -0x10;
          iStack_3c = iVar13;
        }
        iVar11 = iVar11 + 1;
        lVar14 = lVar14 + 0x60;
      } while (iVar11 < iVar1);
    }
    return 4;
  }
  return 0;
}


