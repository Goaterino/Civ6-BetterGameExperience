
void FUN_140199450(longlong param_1,undefined8 *param_2)

{
  longlong lVar1;
  uint uVar2;
  undefined8 *puVar3;
  bool bVar4;
  bool bVar5;
  undefined4 *puVar6;
  undefined8 *puVar7;
  uint uVar8;
  uint *puVar9;
  uint uVar10;
  ulonglong uVar11;
  uint uVar12;
  uint *puVar13;
  ulonglong uVar14;
  uint local_res8 [2];
  undefined1 local_d8 [8];
  uint auStack_d0 [34];
  uint *local_48;
  uint local_40;
  undefined8 local_3c;
  undefined4 local_34;
  
  if (*(undefined8 **)(param_1 + 0x1d0) != param_2) {
    uVar14 = 0;
    if (*(undefined8 **)(param_1 + 0x1d0) != (undefined8 *)0x0) {
      uVar2 = *(uint *)(param_1 + 0x40);
      local_3c = 0;
      local_34 = 0;
      local_48 = (uint *)0x0;
      local_40 = 0;
      local_res8[0] = 0;
      uVar11 = uVar14;
      if (uVar2 != 0) {
        do {
          uVar10 = (uint)uVar11;
          lVar1 = (uVar11 >> 5) * 4;
          uVar8 = 1 << ((byte)uVar11 & 0x1f);
          uVar12 = *(uint *)(lVar1 + *(longlong *)(param_1 + 400)) & uVar8;
          if (((uVar12 == 0) &&
              ((*(uint *)(lVar1 + *(longlong *)(*(longlong *)(param_1 + 0x1d0) + 0x118)) & uVar8) ==
               0)) || ((*(uint *)(*(longlong *)(*(longlong *)(param_1 + 0x1d0) + 0x138) + lVar1) &
                       uVar8) == 0)) {
            bVar4 = false;
          }
          else {
            bVar4 = true;
          }
          if (((uVar12 == 0) && ((*(uint *)(lVar1 + param_2[0x23]) & uVar8) == 0)) ||
             ((*(uint *)(lVar1 + param_2[0x27]) & uVar8) == 0)) {
            bVar5 = true;
          }
          else {
            bVar5 = false;
          }
          if (bVar4) {
            if (bVar5) {
              if ((uVar10 != 4) && (uVar10 != 0x37)) {
                FUN_140197f10(param_1,*(undefined4 *)(uVar11 * 0x20 + *(longlong *)(param_1 + 0x30))
                             );
              }
              puVar6 = (undefined4 *)
                       FUN_1401961c0(&DAT_141b0b380 + (ulonglong)DAT_141b0b304 * 0x200060);
              if (puVar6 != (undefined4 *)0x0) {
                *puVar6 = *(undefined4 *)(uVar11 * 0x20 + *(longlong *)(param_1 + 0x30));
              }
            }
          }
          else if (!bVar5) {
            if (local_3c._4_4_ == local_40) {
              FUN_140198c30(local_d8,local_40 + 1,local_res8,0);
            }
            else if (local_48 + local_3c._4_4_ != (uint *)0x0) {
              local_48[local_3c._4_4_] = uVar10;
            }
            local_3c = CONCAT44(local_3c._4_4_ + 1,(undefined4)local_3c);
          }
          local_res8[0] = uVar10 + 1;
          uVar11 = (ulonglong)local_res8[0];
        } while (local_res8[0] < uVar2);
      }
      uVar11 = (ulonglong)((longlong)(local_48 + (local_3c >> 0x20)) + (3 - (longlong)local_48)) >>
               2;
      if (local_48 + (local_3c >> 0x20) < local_48) {
        uVar11 = uVar14;
      }
      puVar13 = local_48;
      if (uVar11 != 0) {
        do {
          uVar2 = *puVar13;
          puVar9 = &DAT_141b0b380 +
                   (ulonglong)DAT_141b0b304 * 0x200060 +
                   (ulonglong)(*(uint *)(&DAT_14230b480 + (ulonglong)DAT_141b0b304 * 0x800180) & 1)
                   * 0x100020;
          do {
            uVar8 = *puVar9;
            if (0x400000 < uVar8 + 0x80) goto LAB_1401996d6;
            LOCK();
            uVar10 = *puVar9;
            if (uVar8 == uVar10) {
              *puVar9 = uVar8 + 0x80;
              uVar10 = uVar8;
            }
            UNLOCK();
          } while (uVar8 != uVar10);
          puVar6 = (undefined4 *)((longlong)puVar9 + (ulonglong)uVar8 + 0x80);
          if (puVar6 != (undefined4 *)0x0) {
            *puVar6 = 0x80;
            *(undefined8 *)(puVar6 + 1) = 0x1d;
            puVar6[3] = 0;
            if (puVar6 + 4 != (undefined4 *)0x0) {
              puVar6[4] = *(undefined4 *)((ulonglong)uVar2 * 0x20 + *(longlong *)(param_1 + 0x30));
            }
          }
LAB_1401996d6:
          puVar13 = puVar13 + 1;
          uVar14 = uVar14 + 1;
        } while (uVar14 != uVar11);
      }
      if (local_48 != auStack_d0) {
        _aligned_free(local_48);
      }
    }
    puVar3 = *(undefined8 **)(param_1 + 0x1d0);
    *(undefined8 **)(param_1 + 0x1d0) = param_2;
    puVar13 = &DAT_141b0b380 +
              (ulonglong)DAT_141b0b304 * 0x200060 +
              (ulonglong)(*(uint *)(&DAT_14230b480 + (ulonglong)DAT_141b0b304 * 0x800180) & 1) *
              0x100020;
    do {
      uVar2 = *puVar13;
      if (0x400000 < uVar2 + 0x80) {
        return;
      }
      LOCK();
      uVar8 = *puVar13;
      if (uVar2 == uVar8) {
        *puVar13 = uVar2 + 0x80;
        uVar8 = uVar2;
      }
      UNLOCK();
    } while (uVar2 != uVar8);
    puVar6 = (undefined4 *)((longlong)puVar13 + (ulonglong)uVar2 + 0x80);
    if (puVar6 != (undefined4 *)0x0) {
      *puVar6 = 0x80;
      *(undefined8 *)(puVar6 + 1) = 0x1b;
      puVar6[3] = 0;
      puVar7 = (undefined8 *)(puVar6 + 4);
      if (puVar7 != (undefined8 *)0x0) {
        *puVar7 = 0;
        *(undefined8 *)(puVar6 + 6) = 0;
        *puVar7 = *param_2;
        *(undefined8 *)(puVar6 + 6) = *puVar3;
      }
    }
  }
  return;
}


