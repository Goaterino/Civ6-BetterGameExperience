
void FUN_1800127e0(longlong *param_1,byte *param_2)

{
  byte bVar1;
  char cVar2;
  int iVar3;
  int local_res8;
  int local_resc;
  
  bVar1 = (**(code **)(*param_1 + 0x18))();
  param_2[2] = bVar1;
  if (param_1[3] != 0) {
    local_res8 = (int)(short)param_1[5];
    local_resc = (int)*(short *)((longlong)param_1 + 0x2a);
    cVar2 = FUN_18002bc70(param_1[3],&local_res8);
    if (cVar2 != '\0') {
      bVar1 = 2;
      goto LAB_180012826;
    }
  }
  bVar1 = 0;
LAB_180012826:
  *param_2 = *param_2 & 0xfd;
  *param_2 = *param_2 | bVar1;
  bVar1 = FUN_18005f610(param_1);
  param_2[1] = bVar1;
  bVar1 = (**(code **)(*param_1 + 0x20))(param_1);
  param_2[3] = bVar1;
  iVar3 = FUN_180144810(0xffffffff,(int)(short)param_1[5],(int)*(short *)((longlong)param_1 + 0x2a))
  ;
  *param_2 = *param_2 & 0xfe;
  *param_2 = *param_2 | iVar3 == 1;
  bVar1 = FUN_18005ffa0(param_1);
  *param_2 = *param_2 & 0xfb;
  *param_2 = *param_2 | (bVar1 & 1) << 2;
  return;
}


