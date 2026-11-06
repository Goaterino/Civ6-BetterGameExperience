
undefined8 FUN_1806f96b0(longlong param_1)

{
  uint uVar1;
  undefined4 *puVar2;
  char cVar3;
  uint *puVar4;
  longlong lVar5;
  undefined8 uVar6;
  uint local_res10 [2];
  undefined1 local_res18 [16];
  
  puVar4 = (uint *)FUN_1806f8ad0(param_1,1,1);
  uVar1 = *puVar4;
  lVar5 = FUN_18035b360();
  if ((((*(longlong *)(lVar5 + 8) != 0) && (uVar1 < 0x40)) &&
      (*(int *)(*(longlong *)(lVar5 + 8) + (longlong)(int)uVar1 * 4) != -1)) || (uVar1 == 1000)) {
    uVar6 = FUN_18035b360();
    lVar5 = FUN_18035b620(uVar6,uVar1);
    if (lVar5 != 0) {
      cVar3 = thunk_FUN_18001a7b0(param_1,local_res10,2,0,local_res18);
      if (cVar3 != '\0') {
        if (local_res10[0] < *(uint *)(lVar5 + 0x148)) {
          cVar3 = *(char *)((ulonglong)local_res10[0] + *(longlong *)(lVar5 + 0x138));
        }
        else {
          cVar3 = '\0';
        }
        puVar2 = *(undefined4 **)(param_1 + 0x48);
        puVar2[2] = (uint)(cVar3 != '\0');
        *puVar2 = 1;
        *(undefined4 **)(param_1 + 0x48) = puVar2 + 4;
        return 1;
      }
    }
  }
  return 0;
}


