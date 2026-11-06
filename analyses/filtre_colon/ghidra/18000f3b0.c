
int FUN_18000f3b0(int param_1,undefined4 param_2,undefined8 param_3)

{
  undefined4 *puVar1;
  int iVar2;
  longlong lVar3;
  longlong *plVar4;
  undefined4 *puVar5;
  undefined4 uStack_f4;
  undefined4 uStack_f0;
  undefined4 uStack_ec;
  undefined4 local_e8;
  undefined4 uStack_e4;
  undefined4 uStack_e0;
  undefined4 uStack_dc;
  undefined4 local_d8;
  undefined4 uStack_d4;
  undefined4 uStack_d0;
  undefined4 uStack_cc;
  undefined4 local_c8;
  undefined4 uStack_c4;
  undefined4 uStack_c0;
  undefined4 uStack_bc;
  undefined8 local_b8;
  undefined4 local_a8;
  undefined4 uStack_a4;
  undefined4 uStack_a0;
  undefined4 uStack_9c;
  undefined4 local_98;
  undefined4 uStack_94;
  undefined4 uStack_90;
  undefined4 uStack_8c;
  undefined4 local_88;
  undefined4 uStack_84;
  undefined4 uStack_80;
  undefined4 uStack_7c;
  undefined4 local_78;
  undefined4 uStack_74;
  undefined4 uStack_70;
  undefined4 uStack_6c;
  undefined8 local_28;
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined8 local_18;
  
  if (param_1 == 0x77d19dc2) {
    FUN_180012700(param_3,&uStack_f4);
    FUN_180012a20(param_3,&uStack_e0,0);
    FUN_1800127e0(param_3,(longlong)&uStack_d0 + 2);
    FUN_1800126d0(param_3,&local_c8,0);
    puVar1 = DAT_180b8a7e0;
    local_28 = local_b8;
    uStack_a4 = uStack_f4;
    uStack_a0 = uStack_f0;
    uStack_9c = uStack_ec;
    local_98 = local_e8;
    uStack_94 = uStack_e4;
    uStack_90 = uStack_e0;
    uStack_8c = uStack_dc;
    local_88 = local_d8;
    uStack_84 = uStack_d4;
    uStack_80 = uStack_d0;
    uStack_7c = uStack_cc;
    local_78 = local_c8;
    uStack_74 = uStack_c4;
    uStack_70 = uStack_c0;
    uStack_6c = uStack_bc;
    puVar5 = DAT_180b8a7e0;
    local_a8 = param_2;
    if (DAT_180b8a7e0 == (undefined4 *)0x0) {
      FUN_18008c2d0(&DAT_180b8a7d0);
      puVar5 = DAT_180b8a7e0;
    }
    DAT_180b8a7e0 = *(undefined4 **)(puVar5 + 0x12);
    DAT_180b8a7d0 = DAT_180b8a7d0 + 1;
    if (puVar1 == (undefined4 *)0x0) {
      lVar3 = 0x528;
      do {
        *(undefined1 *)(lVar3 + 0x50 + DAT_180b8a7d8) = 1;
        lVar3 = lVar3 + -0x58;
      } while (-1 < lVar3);
    }
    *(undefined1 *)(puVar5 + 0x14) = 0;
    *puVar5 = local_a8;
    puVar5[1] = uStack_a4;
    puVar5[2] = uStack_a0;
    puVar5[3] = uStack_9c;
    puVar5[4] = local_98;
    puVar5[5] = uStack_94;
    puVar5[6] = uStack_90;
    puVar5[7] = uStack_8c;
    puVar5[8] = local_88;
    puVar5[9] = uStack_84;
    puVar5[10] = uStack_80;
    puVar5[0xb] = uStack_7c;
    puVar5[0xc] = local_78;
    puVar5[0xd] = uStack_74;
    puVar5[0xe] = uStack_70;
    puVar5[0xf] = uStack_6c;
    puVar5[0x10] = (undefined4)local_28;
    puVar5[0x11] = local_28._4_4_;
    puVar5[0x12] = uStack_20;
    puVar5[0x13] = uStack_1c;
    *(undefined8 *)(puVar5 + 0x14) = local_18;
    plVar4 = (longlong *)FUN_1800cdfb0(&DAT_180b8a7f0,&DAT_180b8a818);
    *plVar4 = (longlong)puVar5;
    iVar2 = DAT_180b8a818;
    DAT_180b8a818 = DAT_180b8a818 + 1;
    if (DAT_180b8a81c < DAT_180b8a7d0) {
      DAT_180b8a81c = DAT_180b8a7d0;
    }
    return iVar2;
  }
  return -1;
}


