#include <libproc.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
int main(int argc, char** argv) {
  if (argc < 2) { printf("usage: footprint <pid>\n"); return 2; }
  pid_t pid = (pid_t)atoi(argv[1]);
  struct rusage_info_v2 info;
  memset(&info, 0, sizeof(info));
  int r = proc_pid_rusage(pid, RUSAGE_INFO_V2, (rusage_info_t*)&info);
  if (r != 0) { printf("proc_pid_rusage failed: %d (%s)\n", r, strerror(errno)); return 1; }
  printf("footprint(活动监视器'内存')=%.1f MB   resident=%.1f MB\n",
         info.ri_phys_footprint / 1048576.0, info.ri_resident_size / 1048576.0);
  return 0;
}
