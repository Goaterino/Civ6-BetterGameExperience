
undefined8 FUN_180712220(undefined8 param_1)

{
  short sVar1;
  int iVar2;
  undefined4 *puVar3;
  lua_State *plVar4;
  int iVar5;
  int iVar6;
  undefined8 uVar7;
  longlong lVar8;
  longlong lVar9;
  longlong lVar10;
  longlong lVar11;
  ulonglong uVar12;
  longlong *plVar13;
  int iVar14;
  int local_res10 [2];
  int local_68;
  int local_64;
  lua_State *local_58 [2];
  lua_State **local_48;
  undefined1 local_40 [24];
  
  FUN_1809ae590(local_58,param_1,0,0);
  iVar5 = FUN_18007aef0();
  if (-1 < iVar5) {
    uVar7 = FUN_18007af10();
    lVar8 = FUN_18000a2a0(uVar7,iVar5);
    lVar9 = FUN_18007af10();
    iVar14 = 0;
    iVar2 = *(int *)(lVar9 + 0x30);
    if (0 < iVar2) {
      lVar9 = 0;
      do {
        lVar10 = FUN_18007af10();
        if ((iVar14 < 0) || (*(int *)(lVar10 + 0x30) <= iVar14)) {
          plVar13 = (longlong *)0x0;
        }
        else {
          plVar13 = (longlong *)(*(longlong *)(lVar10 + 0x170) + lVar9);
        }
        lVar10 = plVar13[5];
        sVar1 = *(short *)((longlong)plVar13 + 0x2a);
        if ((((*(byte *)(plVar13 + 10) & 1) != 0) ||
            ((lVar11 = (**(code **)(*plVar13 + 0x30))(plVar13), *(char *)(lVar11 + 0x1c) == iVar5 &&
             (iVar6 = FUN_180144810(iVar5,(int)(short)lVar10,(int)sVar1), iVar6 == 1)))) &&
           ((uVar12 = (ulonglong)(uint)(*(int *)(lVar8 + 8) * (int)sVar1 + (int)(short)lVar10),
            0 < *(short *)(*(longlong *)(*(longlong *)(lVar8 + 0x10) + 0x10) + uVar12 * 2) ||
            (*(char *)(uVar12 + *(longlong *)(lVar8 + 0x18)) != '\0')))) {
          local_68 = (int)(short)plVar13[5];
          local_64 = (int)*(short *)((longlong)plVar13 + 0x2a);
          FUN_180114d90(local_res10,iVar5,&local_68,0);
          iVar6 = local_res10[0] >> 8;
          if (iVar6 < 0) {
            local_48 = local_58;
            FUN_1809ae190(local_40);
            FUN_1809adf70();
            plVar4 = local_58[0];
            if (local_58[0] != (lua_State *)0x0) {
              if (*(ulonglong *)(local_58[0] + 0x58) < *(longlong *)(local_58[0] + 0x48) + 0x10U) {
                hks::CallStack::growApiStack((CallStack *)(local_58[0] + 0x18),local_58[0],1);
              }
              puVar3 = *(undefined4 **)(plVar4 + 0x48);
              *puVar3 = 3;
              *(double *)(puVar3 + 2) = (double)iVar14;
              *(undefined4 **)(plVar4 + 0x48) = puVar3 + 4;
              FUN_1809aeaa0(local_40,plVar4,0xffffffff);
              *(longlong *)(plVar4 + 0x48) = *(longlong *)(plVar4 + 0x48) + -0x10;
            }
            FUN_1809ae060();
            FUN_1805b8fa0(local_48,local_40,iVar6);
            FUN_1809ae1b0(local_40);
          }
        }
        iVar14 = iVar14 + 1;
        lVar9 = lVar9 + 0x60;
      } while (iVar14 < iVar2);
    }
  }
  FUN_1809ae9a0(local_58);
  FUN_1809ae1b0(local_58);
  return 1;
}


