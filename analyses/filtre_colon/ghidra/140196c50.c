
void FUN_140196c50(longlong param_1,int param_2,int param_3,longlong *param_4,longlong *param_5,
                  int param_6,int param_7)

{
  int *piVar1;
  int *piVar2;
  longlong lVar3;
  uint uVar4;
  char cVar5;
  int iVar6;
  int iVar7;
  int iVar8;
  undefined8 uVar9;
  uint uVar10;
  int *piVar11;
  undefined4 *puVar12;
  uint uVar13;
  longlong lVar14;
  ulonglong uVar15;
  undefined8 *puVar16;
  uint *puVar17;
  uint uVar18;
  ulonglong uVar19;
  undefined8 *puVar20;
  ulonglong uVar21;
  int iVar22;
  ulonglong uVar23;
  uint uVar24;
  undefined4 *puVar25;
  undefined8 local_a0;
  uint uStack_98;
  undefined4 uStack_94;
  uint uStack_90;
  undefined4 local_8c;
  uint local_88;
  uint local_84;
  uint local_80;
  uint local_7c;
  undefined8 local_78;
  undefined8 *local_70;
  ulonglong uStack_68;
  undefined4 local_60;
  undefined4 uStack_5c;
  uint uStack_58;
  undefined4 uStack_54;
  
  if ((*(char *)(param_1 + 0x1d8) != '\0') && (-1 < param_3)) {
    lVar14 = *(longlong *)(param_1 + 0x58);
    uVar18 = *(uint *)(param_1 + 0x48);
    local_80 = 0xffffffff;
    uVar10 = FUN_14004f970(uVar18,lVar14,param_2,*(undefined4 *)(param_1 + 0x50));
    uVar15 = (ulonglong)uVar10;
    uVar24 = 0xffffffff;
    if (uVar10 != 0xffffffff) {
      iVar22 = *(int *)(lVar14 + uVar15 * 4);
      while (uVar24 = 0xffffffff, iVar22 != -1) {
        if (*(int *)(lVar14 + uVar15 * 4) == param_2) {
          uVar24 = *(uint *)(*(longlong *)(param_1 + 0x60) + uVar15 * 4);
          local_80 = uVar24;
          break;
        }
        uVar15 = (ulonglong)((int)uVar15 + 1) % (ulonglong)uVar18;
        iVar22 = *(int *)(lVar14 + uVar15 * 4);
      }
    }
    if (uVar24 < *(uint *)(param_1 + 0x40)) {
      switch(uVar24) {
      case 0x26:
        param_6 = 0x29b5e70d;
        break;
      default:
        if (param_6 == 0) {
          param_6 = *(int *)(*(longlong *)(param_1 + 0x1d0) + 0x110);
        }
        break;
      case 0x2a:
        param_6 = 0x6c042f7d;
        break;
      case 0x30:
        param_6 = 0x41c5f989;
        break;
      case 0x33:
        param_6 = -0x6d064db2;
        break;
      case 0x34:
        param_6 = -0x1e622161;
        break;
      case 0x35:
        param_6 = 0x6aa681b7;
        break;
      case 0x38:
        param_6 = 0x9a6b97a;
      }
      uVar18 = *(uint *)(param_4 + 2);
      uVar19 = 0;
      uVar24 = 0;
      uStack_94 = 0;
      uStack_90 = 0;
      local_8c = 0;
      local_a0 = (undefined8 *)0x0;
      local_84 = *(uint *)(param_5 + 2);
      uStack_98 = 0;
      local_88 = 0;
      uVar15 = uVar19;
      uVar23 = uVar19;
      local_7c = uVar18;
      if (uVar18 != 0) {
        do {
          piVar11 = (int *)FUN_1401964c0(&DAT_141b0b380 + (ulonglong)DAT_141b0b304 * 0x200060);
          if (piVar11 == (int *)0x0) goto LAB_14019715c;
          *piVar11 = param_6;
          uVar21 = 0;
          piVar11[1] = param_2;
          piVar11[2] = param_3;
          piVar11[0xe4] = param_7;
          iVar22 = (int)uVar15;
          do {
            uVar24 = (uint)uVar15;
            uVar10 = uVar24 + ((int)uVar23 * 0x40 - iVar22);
            if (uVar18 <= uVar10) break;
            FUN_1400f2090(&local_78,*(undefined4 *)(*param_4 + (ulonglong)uVar10 * 4));
            uVar9 = local_78;
            uVar18 = (uint)uVar19;
            if (uVar24 == uVar18) {
              uVar13 = uVar18 + 1;
              local_60 = (undefined4)local_a0;
              uStack_5c = local_a0._4_4_;
              uStack_58 = uStack_98;
              uStack_54 = uStack_94;
              if (uVar13 != uVar18) {
                local_70 = (undefined8 *)0x0;
                uStack_68 = 0;
                if (uVar18 < uVar13) {
                  uVar4 = uVar18 * 2;
                  if (uVar18 * 2 < uVar13) {
                    uVar4 = uVar13;
                  }
                  uStack_68 = (ulonglong)uVar4;
                  local_a0 = (undefined8 *)
                             FUN_14098ff60(uVar4 * 8,0x10,
                                           "G:\\BuildAgent\\work\\26d31e7d2eca7bbe\\CivTech\\Libs\\Platform\\Common\\Platform_Allocators.h"
                                           ,0x9f,1,0);
                  puVar20 = (undefined8 *)CONCAT44(uStack_5c,local_60);
                  uStack_98 = (uint)uStack_68;
                  uStack_94 = (undefined4)(uStack_68 >> 0x20);
                  local_70 = local_a0;
                  if (puVar20 != local_a0) {
                    if (uVar24 != 0) {
                      puVar16 = local_a0;
                      uVar19 = uVar15;
                      do {
                        if (puVar16 != (undefined8 *)0x0) {
                          *puVar16 = *(undefined8 *)
                                      (((longlong)puVar20 - (longlong)local_a0) + (longlong)puVar16)
                          ;
                        }
                        puVar16 = puVar16 + 1;
                        uVar19 = uVar19 - 1;
                      } while (uVar19 != 0);
                    }
                    _aligned_free(puVar20);
                  }
                  uVar19 = (ulonglong)uStack_98;
                }
              }
            }
            if (local_a0 + uVar15 != (undefined8 *)0x0) {
              local_a0[uVar15] = uVar9;
            }
            uVar15 = (ulonglong)(uVar24 + 1);
            *(undefined8 *)(piVar11 + uVar21 * 2 + 4) = local_78;
            cVar5 = (char)uVar21;
            uVar24 = (int)uVar21 + 1;
            uVar21 = (ulonglong)uVar24;
            *(char *)(piVar11 + 3) = cVar5 + '\x01';
            uVar18 = local_7c;
          } while (uVar24 < 0x40);
          uVar24 = (uint)uVar19;
          local_88 = local_88 + 1;
          uVar23 = (ulonglong)local_88;
          uStack_90 = (uint)uVar15;
        } while (uVar10 < uVar18);
      }
      iVar22 = 0;
      uVar18 = local_84;
      if (local_84 != 0) {
        do {
          uVar15 = 0;
          puVar17 = &DAT_141b0b380 +
                    (ulonglong)DAT_141b0b304 * 0x200060 +
                    (ulonglong)(*(uint *)(&DAT_14230b480 + (ulonglong)DAT_141b0b304 * 0x800180) & 1)
                    * 0x100020;
          do {
            uVar24 = *puVar17;
            if (0x400000 < uVar24 + 0x400) goto LAB_14019715c;
            LOCK();
            uVar10 = *puVar17;
            if (uVar24 == uVar10) {
              *puVar17 = uVar24 + 0x400;
              uVar10 = uVar24;
            }
            UNLOCK();
          } while (uVar24 != uVar10);
          puVar25 = (undefined4 *)((ulonglong)uVar24 + 0x80 + (longlong)puVar17);
          if (puVar25 == (undefined4 *)0x0) goto LAB_14019715c;
          *puVar25 = 0x400;
          *(undefined8 *)(puVar25 + 1) = 0x24;
          puVar25[3] = 0;
          piVar11 = puVar25 + 4;
          if (piVar11 == (int *)0x0) goto LAB_14019715c;
          *piVar11 = 0;
          puVar20 = (undefined8 *)(puVar25 + 8);
          puVar25[5] = 0xffffffff;
          lVar14 = 0x40;
          *(undefined2 *)(puVar25 + 7) = 0;
          do {
            *puVar20 = 0;
            puVar20 = puVar20 + 1;
            lVar14 = lVar14 + -1;
          } while (lVar14 != 0);
          lVar14 = 0x10;
          puVar12 = puVar25 + 0x8c;
          do {
            *(undefined8 *)(puVar12 + -3) = 0;
            *(undefined8 *)(puVar12 + -1) = 0;
            puVar12 = puVar12 + 6;
            lVar14 = lVar14 + -1;
          } while (lVar14 != 0);
          puVar25[0xe8] = 0xffffffff;
          *piVar11 = param_6;
          puVar25[5] = param_2;
          puVar25[6] = param_3;
          puVar25[0xe8] = param_7;
          uVar24 = uStack_98;
          do {
            uVar10 = (int)uVar15 + iVar22 * 0x10;
            if (uVar18 <= uVar10) break;
            lVar14 = (ulonglong)uVar10 * 0x18;
            lVar3 = *param_5;
            piVar1 = (int *)(lVar14 + lVar3);
            iVar6 = piVar1[1];
            iVar7 = piVar1[2];
            iVar8 = piVar1[3];
            piVar2 = piVar11 + (uVar15 + 0x16) * 6;
            *piVar2 = *piVar1;
            piVar2[1] = iVar6;
            piVar2[2] = iVar7;
            piVar2[3] = iVar8;
            *(undefined8 *)(puVar25 + (uVar15 + 0x16) * 6 + 8) =
                 *(undefined8 *)(lVar14 + 0x10 + lVar3);
            *(char *)((longlong)puVar25 + 0x1d) = (char)uVar15 + '\x01';
            puVar20 = (undefined8 *)(*param_5 + 4 + lVar14);
            if (uStack_90 == uVar24) {
              FUN_14057dce0(&local_a0,uVar24 + 1,puVar20,0);
              uVar18 = local_84;
              uVar24 = uStack_98;
            }
            else if (local_a0 + uStack_90 != (undefined8 *)0x0) {
              local_a0[uStack_90] = *puVar20;
            }
            uStack_90 = uStack_90 + 1;
            uVar13 = (int)uVar15 + 1;
            uVar15 = (ulonglong)uVar13;
          } while (uVar13 < 0x10);
          iVar22 = iVar22 + 1;
        } while (uVar10 < uVar18);
      }
      uVar18 = uStack_90;
      puVar20 = (undefined8 *)(*(longlong *)(param_1 + 0x30) + 8 + (ulonglong)local_80 * 0x20);
      if (&local_a0 != puVar20) {
        _aligned_free((void *)*puVar20);
        *puVar20 = local_a0;
        local_a0 = (undefined8 *)0x0;
        *(uint *)(puVar20 + 1) = uVar24;
        *(uint *)(puVar20 + 2) = uVar18;
      }
LAB_14019715c:
      _aligned_free(local_a0);
    }
  }
  return;
}


