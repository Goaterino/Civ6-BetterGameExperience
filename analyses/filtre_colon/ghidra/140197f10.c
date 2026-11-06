
void FUN_140197f10(longlong param_1,int param_2)

{
  uint uVar1;
  char cVar2;
  uint uVar3;
  int *piVar4;
  ulonglong uVar5;
  uint uVar6;
  ulonglong uVar7;
  int iVar8;
  longlong lVar9;
  
  if (*(char *)(param_1 + 0x1d8) != '\0') {
    uVar1 = *(uint *)(param_1 + 0x48);
    lVar9 = *(longlong *)(param_1 + 0x58);
    uVar3 = FUN_14004f970(uVar1,lVar9,param_2,*(undefined4 *)(param_1 + 0x50));
    uVar7 = (ulonglong)uVar3;
    uVar6 = 0xffffffff;
    if (uVar3 != 0xffffffff) {
      iVar8 = *(int *)(lVar9 + uVar7 * 4);
      while (uVar6 = 0xffffffff, iVar8 != -1) {
        if (*(int *)(lVar9 + uVar7 * 4) == param_2) {
          uVar6 = *(uint *)(*(longlong *)(param_1 + 0x60) + uVar7 * 4);
          break;
        }
        uVar7 = (ulonglong)((int)uVar7 + 1) % (ulonglong)uVar1;
        iVar8 = *(int *)(lVar9 + uVar7 * 4);
      }
    }
    if (uVar6 < *(uint *)(param_1 + 0x40)) {
      lVar9 = (ulonglong)uVar6 * 0x20 + *(longlong *)(param_1 + 0x30);
      if (*(char *)(lVar9 + 4) == '\0') {
        uVar1 = *(uint *)(lVar9 + 0x18);
        if (uVar1 != 0) {
          uVar7 = 0;
          iVar8 = 0;
          do {
            piVar4 = (int *)FUN_1401960a0();
            if (piVar4 == (int *)0x0) break;
            *(undefined1 *)(piVar4 + 0x22) = 0;
            *piVar4 = param_2;
            if ((uint)uVar7 < uVar1) {
              uVar5 = 0;
              do {
                uVar6 = (int)uVar5 + iVar8 * 0x10;
                uVar7 = (ulonglong)uVar6;
                if (uVar1 <= uVar6) break;
                *(undefined8 *)(piVar4 + uVar5 * 2 + 2) =
                     *(undefined8 *)(*(longlong *)(lVar9 + 8) + uVar7 * 8);
                cVar2 = (char)uVar5;
                uVar6 = (int)uVar5 + 1;
                uVar5 = (ulonglong)uVar6;
                *(char *)(piVar4 + 1) = cVar2 + '\x01';
              } while (uVar6 < 0x10);
            }
            iVar8 = iVar8 + 1;
          } while ((uint)uVar7 < uVar1);
        }
      }
      else {
        piVar4 = (int *)FUN_1401960a0(&DAT_141b0b380 + (ulonglong)DAT_141b0b304 * 0x200060);
        if (piVar4 != (int *)0x0) {
          *(undefined1 *)(piVar4 + 0x22) = 1;
          *piVar4 = param_2;
        }
      }
      *(undefined4 *)(lVar9 + 0x18) = 0;
    }
  }
  return;
}


