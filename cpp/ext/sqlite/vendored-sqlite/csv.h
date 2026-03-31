#ifndef SQLITE_CSV_H
#define SQLITE_CSV_H

#include <sqlite3.h>

#ifdef __cplusplus
extern "C" {
#endif

int sqlite3_csv_init(sqlite3* db, char** pzErrMsg,
                     const sqlite3_api_routines* pApi);

#ifdef __cplusplus
}
#endif

#endif
