#include "memory_limit.h"
#include <iostream>
#include <cstdlib>

#if defined(_WIN32) || defined(_WIN64)
#include <windows.h>
#else
#include <sys/resource.h>
#if defined(__APPLE__)
#include <sys/sysctl.h>
#else
#include <sys/sysinfo.h>
#endif
#endif

unsigned long get_total_memory()
{
#if defined(_WIN32) || defined(_WIN64)
  MEMORYSTATUSEX status;
  status.dwLength = sizeof(status);
  GlobalMemoryStatusEx(&status);
  return status.ullTotalPhys;
#elif defined(__APPLE__)
  int mib[2] = {CTL_HW, HW_MEMSIZE};
  unsigned long total_memory;
  size_t length = sizeof(total_memory);
  if (sysctl(mib, 2, &total_memory, &length, NULL, 0) != 0)
  {
    std::cerr << "Failed to get system memory size" << std::endl;
  }
  return total_memory;
#else // Linux
  struct sysinfo info;
  if (sysinfo(&info) != 0)
  {
    std::cerr << "Failed to get system info" << std::endl;
  }
  return info.totalram * info.mem_unit;
#endif
}

void set_memory_limit()
{
  unsigned long total_memory = get_total_memory();
  unsigned long limit_50_percent = total_memory / 2;
  unsigned long max_limit = 20UL * 1024 * 1024 * 1024;
  unsigned long memory_limit = std::min(max_limit, limit_50_percent);

#if defined(_WIN32) || defined(_WIN64)
  SIZE_T minimumWorkingSetSize = memory_limit;
  SIZE_T maximumWorkingSetSize = memory_limit;
  if (!SetProcessWorkingSetSize(GetCurrentProcess(), minimumWorkingSetSize, maximumWorkingSetSize))
  {
    std::cerr << "Failed to set memory limit" << std::endl;
  }
#else
  struct rlimit rl;
  rl.rlim_cur = memory_limit;
  rl.rlim_max = memory_limit;
  if (setrlimit(RLIMIT_AS, &rl) != 0)
  {
    std::cerr << "Failed to set memory limit" << std::endl;
  }
#endif
}
