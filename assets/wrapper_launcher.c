#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <mach-o/dyld.h>

/* DYLD insertion is process-local. Preserve caller libraries and forward argv. */
int main(int argc, char *argv[]) {
    (void)argc;
    uint32_t size = 0;
    _NSGetExecutablePath(NULL, &size);
    char *path = malloc(size);
    if (!path || _NSGetExecutablePath(path, &size) != 0) return 1;
    char *resolved = realpath(path, NULL);
    free(path);
    if (!resolved) { perror("resolve launcher"); return 1; }
    char *slash = strrchr(resolved, '/');
    if (!slash) { free(resolved); return 1; }
    size_t directory = (size_t)(slash - resolved);
    const char *existing = getenv("DYLD_INSERT_LIBRARIES");
    size_t length = directory + sizeof("/libfonthook.dylib") + (existing ? strlen(existing) + 1 : 0);
    char *libraries = malloc(length);
    char *original = malloc(strlen(resolved) + sizeof(".orig"));
    if (!libraries || !original) { free(libraries); free(original); free(resolved); return 1; }
    snprintf(libraries, length, "%.*s/libfonthook.dylib%s%s", (int)directory, resolved,
             existing && *existing ? ":" : "", existing && *existing ? existing : "");
    if (memchr(resolved, ':', directory) != NULL) {
        fputs("launcher directory contains a DYLD path separator\n", stderr);
        free(libraries); free(original); free(resolved); return 1;
    }
    snprintf(original, strlen(resolved) + sizeof(".orig"), "%s.orig", resolved);
    if (setenv("DYLD_INSERT_LIBRARIES", libraries, 1) != 0) { perror("set DYLD_INSERT_LIBRARIES"); }
    else execv(original, argv);
    perror("execute original binary");
    free(libraries); free(original); free(resolved);
    return 1;
}
