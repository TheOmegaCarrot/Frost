#ifndef SQLITE_VENDORED_EXTENSIONS_H
#define SQLITE_VENDORED_EXTENSIONS_H

#include <sqlite3.h>

#ifdef __cplusplus
extern "C" {
#endif

int sqlite3_csv_init(sqlite3* db, char** pzErrMsg,
                     const sqlite3_api_routines* pApi);

int sqlite3_fileio_init(sqlite3* db, char** pzErrMsg,
                        const sqlite3_api_routines* pApi);

int sqlite3_regexp_init(sqlite3* db, char** pzErrMsg,
                        const sqlite3_api_routines* pApi);

int sqlite3_series_init(sqlite3* db, char** pzErrMsg,
                        const sqlite3_api_routines* pApi);

int sqlite3_shathree_init(sqlite3* db, char** pzErrMsg,
                          const sqlite3_api_routines* pApi);

int sqlite3_sqlar_init(sqlite3* db, char** pzErrMsg,
                       const sqlite3_api_routines* pApi);

int sqlite3_uuid_init(sqlite3* db, char** pzErrMsg,
                      const sqlite3_api_routines* pApi);

int sqlite3_zipfile_init(sqlite3* db, char** pzErrMsg,
                         const sqlite3_api_routines* pApi);

#ifdef __cplusplus
}
#endif

#endif
