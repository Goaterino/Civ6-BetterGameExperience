
undefined8 FUN_180144810(int param_1,undefined4 param_2,undefined4 param_3)

{
  short sVar1;
  int iVar2;
  char cVar3;
  int iVar4;
  int iVar5;
  longlong *plVar6;
  longlong *plVar7;
  longlong *plVar8;
  undefined8 uVar9;
  longlong lVar10;
  longlong lVar11;
  short *psVar12;
  int iVar13;
  undefined1 local_res20 [8];
  int local_38;
  int local_34;
  
  plVar6 = (longlong *)FUN_180044d40();
  if (((plVar6 != (longlong *)0x0) &&
      (plVar7 = (longlong *)(**(code **)(*plVar6 + 0x58))(plVar6,param_2,param_3),
      plVar7 != (longlong *)0x0)) && ((*(byte *)((longlong)plVar7 + 0x3a) & 8) == 0)) {
    sVar1 = *(short *)((longlong)plVar7 + 0x3c);
    if (sVar1 != -1) {
      plVar8 = (longlong *)FUN_1807d0aa0();
      if ((ulonglong)(uint)(int)sVar1 < (ulonglong)(plVar8[1] - *plVar8 >> 3)) {
        lVar10 = *(longlong *)(*plVar8 + (ulonglong)(uint)(int)sVar1 * 8);
      }
      else {
        lVar10 = 0;
      }
      if ((*(byte *)(lVar10 + 0xc2) & 2) == 0) {
        return 0;
      }
    }
    uVar9 = (**(code **)(*plVar7 + 0x30))(plVar7);
    iVar4 = FUN_18005e720(uVar9);
    if (iVar4 == -1) {
      if (param_1 == -1) {
        lVar10 = (**(code **)(*plVar7 + 0x30))(plVar7);
        if (*(char *)(lVar10 + 0x1c) != -1) {
          return 0;
        }
      }
      else {
        lVar10 = FUN_180044f00(param_1);
        lVar11 = (**(code **)(*plVar7 + 0x30))(plVar7);
        if ((*(char *)(lVar11 + 0x1c) != -1) &&
           (lVar11 = (**(code **)(*plVar7 + 0x30))(plVar7),
           (int)*(char *)(lVar11 + 0x1c) != *(int *)(lVar10 + 0xd8))) {
          return 0;
        }
      }
      cVar3 = FUN_180060820(plVar7);
      if (cVar3 == '\0') {
        plVar8 = (longlong *)FUN_1803638b0();
        psVar12 = (short *)(**(code **)(*plVar8 + 0x50))(plVar8,local_res20,plVar7);
        if ((*psVar12 == -1) || (*(int *)(psVar12 + 2) == -1)) {
          iVar4 = *(int *)(DAT_180b8aa70 + 300);
          iVar13 = -iVar4;
          do {
            iVar2 = -iVar4;
            if (iVar4 < iVar13) {
              return 1;
            }
            for (; iVar2 <= iVar4; iVar2 = iVar2 + 1) {
              local_38 = (int)(short)plVar7[5];
              local_34 = (int)*(short *)((longlong)plVar7 + 0x2a);
              plVar8 = (longlong *)FUN_180016010(plVar6,&local_38,iVar13,iVar2,iVar4);
              if (plVar8 != (longlong *)0x0) {
                uVar9 = (**(code **)(*plVar8 + 0x30))(plVar8);
                cVar3 = FUN_18005e770(uVar9);
                if ((cVar3 != '\0') &&
                   ((plVar8[2] == plVar7[2] ||
                    (iVar5 = FUN_180015ec0(plVar6,(int)(short)plVar8[5],
                                           (int)*(short *)((longlong)plVar8 + 0x2a),
                                           (int)(short)plVar7[5],
                                           (int)*(short *)((longlong)plVar7 + 0x2a)), iVar5 < iVar4)
                    ))) {
                  return 0;
                }
              }
            }
            iVar13 = iVar13 + 1;
          } while( true );
        }
      }
    }
  }
  return 0;
}


