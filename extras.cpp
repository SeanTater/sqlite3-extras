#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <string>
#include <sstream>
#include <boost/regex.hpp>
#include <sqlite3ext.h>
SQLITE_EXTENSION_INIT1

#ifdef REDUCED_PRECISION
typedef float real;
#else
typedef double real;
#endif

/**
  Get a string argument from SQLite. It assumes you already know it's there.
  You can check that with argc() or trust SQLite to do that. */
const char *getSQLiteString(
    sqlite3_context *ctx,
    sqlite3_value *arg,
    const std::string& func,
    const std::string& name) {
  const char *value = (const char *) sqlite3_value_text(arg);
  if (value) {
    return value;
  } else {
    sqlite3_result_error(ctx, (func + "(): missing " + name).data(), -1);
    return NULL;
  }
}


extern "C" {
  /**
   * Regular Expressions
   **/


  /**
   * @brief Match a regular expression, giving a boolean.
   */
  void match(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
      const char *re = getSQLiteString(ctx, argv[0], "match", "regular expression");
      const char *subject = getSQLiteString(ctx, argv[1], "match", "subject");
      if (!re || !subject) return;

      // Catch all for regex errors and API cleanliness
      try {
        boost::regex regex(re);
        sqlite3_result_int(ctx, boost::regex_match(subject, regex));
      } catch (const boost::regex_error& e) {
        sqlite3_result_error(ctx, e.what(), -1);
      }

      return;
  }

  /**
   * @brief Search a string with a regex.
   *
   * This differs from match. See Boost::regex for more information.
   */
  void search(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    const char *re = getSQLiteString(ctx, argv[0], "search", "regular expression");
    const char *subject = getSQLiteString(ctx, argv[1], "search", "subject");
    if (!re || !subject) return;

    // Catch all for regex errors and API cleanliness
    try {
      boost::regex regex(re);
      sqlite3_result_int(ctx, boost::regex_search(std::string(subject), regex));
    } catch (const boost::regex_error& e) {
      sqlite3_result_error(ctx, e.what(), -1);
    }
    return;
  }

  /**
   * @brief Substitute regex matches with a formatted string.
   * For more information about the string format, see Boost::regex or the README.
   */
  void sub(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    const char *re = getSQLiteString(ctx, argv[0], "sub", "regular expression");
    const char *format = getSQLiteString(ctx, argv[1], "sub", "format string");
    const char *subject = getSQLiteString(ctx, argv[2], "sub", "subject");
    if (!re || !format || !subject) return;
    // Catch all for regex errors and API cleanliness
    try {
      boost::regex regex(re);
      std::string replaced = boost::regex_replace(std::string(subject), regex, format);
      sqlite3_result_text(ctx, replaced.data(), -1, SQLITE_TRANSIENT);
    } catch (const boost::regex_error& e) {
      sqlite3_result_error(ctx, e.what(), -1);
    }
    return;
  }


  /*****************************************************************************
   * Math
   */


   /** Helper functions */
  /** Perform a unary operator on either a scalar or vector */
  void vunop(sqlite3_context *ctx, sqlite3_value *arg, std::function<real(real)> unop) {
   switch (sqlite3_value_type(arg)) {
     case SQLITE_INTEGER:
     case SQLITE_FLOAT:
       sqlite3_result_double(ctx, unop(sqlite3_value_double(arg)));
       break;
     case SQLITE_BLOB: {
       int len = sqlite3_value_bytes(arg) / sizeof(real);
       real *vec = (real*) sqlite3_value_blob(arg);
       real result_vec[len];
       for (int i=0; i<len; i++) {
         result_vec[i] = unop(vec[i]);
       }
       sqlite3_result_blob(ctx, result_vec, len*sizeof(real), SQLITE_TRANSIENT);
       break;
     }
     default:
       sqlite3_result_error(ctx,
         "Invalid value type for vector/scalar operation. " \
         "Possible causes:\n" \
         "\tPerforming operations on an empty vector, \n"\
         "\tUsing text as a vector or scalar (convert them first with cast() or vread()),\n"\
         "\tNot space-separating values for vread().", -1);
      break;
   }
  }

  // Get the length of a scalar or vector, but scalars are -1 to distinguish them.
  int vecOrScalarLen(sqlite3_value *arg) {
    switch (sqlite3_value_type(arg)) {
      case SQLITE_INTEGER:
      case SQLITE_FLOAT:
        return -1;
      case SQLITE_BLOB:
        return sqlite3_value_bytes(arg) / sizeof(real);
      default:
        return 0;
    }
  }

  // Run a binary operation on two real vectors.
  // It's just choosing a type that makes this operation kinda hairy.
  void vbinop(sqlite3_context *ctx, sqlite3_value **argv, std::function<real(real, real)> binop) {
    int left_len = vecOrScalarLen(argv[0]);
    int right_len = vecOrScalarLen(argv[1]);

    if (left_len == 0 || right_len == 0) {
      // Error
      sqlite3_result_error(ctx,
        "Invalid value type for vector/scalar operation. " \
        "Possible causes:\n" \
        "\tPerforming operations on an empty vector, \n"\
        "\tUsing text as a vector or scalar (convert them first with cast() or vread()),\n"\
        "\tNot space-separating values for vread().", -1);
      return;
    } else if (left_len == -1 && right_len == -1) {
      // Scalar-scalar op
      sqlite3_result_double(ctx, binop(
        sqlite3_value_double(argv[0]),
        sqlite3_value_double(argv[1])
      ));
    } else if (left_len == -1) {
      // Scalar-vector op
      real left = sqlite3_value_double(argv[0]);
      real *right_vec = (real*) sqlite3_value_blob(argv[1]);
      real result_vec[right_len];
      for (int i=0; i<right_len; i++) {
        result_vec[i] = binop(left, right_vec[i]);
      }
      sqlite3_result_blob(ctx, result_vec, right_len*sizeof(real), SQLITE_TRANSIENT);
    } else if (right_len == -1) {
      // Vector-scalar op
      real *left_vec = (real*) sqlite3_value_blob(argv[0]);
      real right = sqlite3_value_double(argv[1]);
      real result_vec[left_len];
      for (int i=0; i<left_len; i++) {
        result_vec[i] = binop(left_vec[i], right);
      }
      sqlite3_result_blob(ctx, result_vec, left_len*sizeof(real), SQLITE_TRANSIENT);
    } else {
      // Vector-vector op
      int len = fmin(left_len, right_len);
      real *left_vec = (real*) sqlite3_value_blob(argv[0]);
      real *right_vec = (real*) sqlite3_value_blob(argv[1]);
      real result_vec[len];
      for (int i=0; i<len; i++) {
        result_vec[i] = binop(left_vec[i], right_vec[i]);
      }
      sqlite3_result_blob(ctx, result_vec, len*sizeof(real), SQLITE_TRANSIENT);
    }
  }


   void sqlsin(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], sin);
   }
   void sqlasin(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], asin);
   }
   void sqlcos(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], cos);
   }
   void sqlacos(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], acos);
   }
   void sqltan(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], tan);
   }
   void sqlatan(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], atan);
   }
   void sqllog(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], log);
   }
   void sqlexp(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], exp);
   }
   void sqlpow(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vbinop(ctx, argv, pow);
   }
   void sqlsqrt(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], sqrt);
   }

  // Create a new zero vector
  void sqlvzero(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    int len = sqlite3_value_int(argv[0]);

    real result_vec[len];
    for (int i=0; i<len; i++) {
      result_vec[i] = 0;
    }
    sqlite3_result_blob(ctx, result_vec, len*sizeof(real), SQLITE_TRANSIENT);
  }

  // Create a new one vector
  void sqlvone(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    int len = sqlite3_value_int(argv[0]);

    real result_vec[len];
    for (int i=0; i<len; i++) {
      result_vec[i] = 1;
    }
    sqlite3_result_blob(ctx, result_vec, len*sizeof(real), SQLITE_TRANSIENT);
  }

  // Add two vectors
  void sqladd(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    vbinop(ctx, argv, [](real a, real b){ return a + b; });
  }
  // Subtract the second vector from the first
  void sqlsubtract(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    vbinop(ctx, argv, [](real a, real b){ return a - b; });
  }
  // Multiply two vectors
  void sqlmult(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    vbinop(ctx, argv, [](real a, real b){ return a * b; });
  }
  // Divide two vectors
  void sqldiv(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    vbinop(ctx, argv, [](real a, real b){ return a / b; });
  }

  // Compute the sum of the elements of a vector: It's not the most numerically
  // stable way though.
  void sqlvsum(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    if (sqlite3_value_type(argv[0]) != SQLITE_BLOB) {
      sqlite3_result_error(ctx, "Vector sum requires a vector.", -1);
      return;
    }
    int len = sqlite3_value_bytes(argv[0]) / sizeof(real);
    real *vec = (real*) sqlite3_value_blob(argv[0]);
    real end = 0.0;
    for (int i=0; i<len; i++) {
      end += vec[i];
    }
    sqlite3_result_double(ctx, end);
  }

  // Compute the product of the elements of a vector: It's not the most
  // numerically stable way though.
  void sqlvprod(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    if (sqlite3_value_type(argv[0]) != SQLITE_BLOB) {
      sqlite3_result_error(ctx, "Vector product requires a vector.", -1);
      return;
    }
    int len = sqlite3_value_bytes(argv[0]) / sizeof(real);
    real *vec = (real*) sqlite3_value_blob(argv[0]);
    real end = 0.0;
    for (int i=0; i<len; i++) {
      end *= vec[i];
    }
    sqlite3_result_double(ctx, end);
  }

  // Compute the dot product: It's not the most numerically stable way though.
  void sqldot(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    if (sqlite3_value_type(argv[0]) != SQLITE_BLOB
        || sqlite3_value_type(argv[1]) != SQLITE_BLOB) {
      sqlite3_result_error(ctx, "Dot product requires two vectors.", -1);
      return;
    }
    int len = fmin(sqlite3_value_bytes(argv[0]), sqlite3_value_bytes(argv[1]))
              / sizeof(real);

    real *a = (real*) sqlite3_value_blob(argv[0]);
    real *b = (real*) sqlite3_value_blob(argv[1]);
    real end = 0.0;
    for (int i=0; i<len; i++) {
      end += a[i] * b[i];
    }
    sqlite3_result_double(ctx, end);
  }

  // Compute the cosine similarity between two vectors
  void sqlcossim(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    if (sqlite3_value_type(argv[0]) != SQLITE_BLOB
        || sqlite3_value_type(argv[1]) != SQLITE_BLOB) {
      sqlite3_result_error(ctx, "Cossim requires two vectors.", -1);
      return;
    }
    int len = fmin(sqlite3_value_bytes(argv[0]), sqlite3_value_bytes(argv[1]))
              / sizeof(real);

    real *a = (real*) sqlite3_value_blob(argv[0]);
    real *b = (real*) sqlite3_value_blob(argv[1]);

    real asq = 0.0;
    #pragma omp simd
    for (int i=0; i<len; i++) asq += a[i] * a[i];

    real bsq = 0.0;
    #pragma omp simd
    for (int i=0; i<len; i++) bsq += b[i] * b[i];


    real absq = 0.0;
    #pragma omp simd
    for (int i=0; i<len; i++) absq += a[i] * b[i];

    sqlite3_result_double(ctx, absq / (sqrt(asq) * sqrt(bsq)));
  }

  // Read a vector from a space separated string
  void sqlvread(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    const char *text = getSQLiteString(ctx, argv[0], "vread", "space separated floating point values");
    std::string str(text);
    std::stringstream stream(str);
    std::vector<real> vec;
    for (real item; stream >> item;) {
      vec.push_back(item);
    }
    sqlite3_result_blob(ctx, &vec.front(), vec.size()*sizeof(real), SQLITE_TRANSIENT);
  }

  // Write a vector to a space separated string
  void sqlvshow(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    int len = sqlite3_value_bytes(argv[0]) / sizeof(real);
    real *vec = (real*) sqlite3_value_blob(argv[0]);
    std::stringstream stream;
    for (int i=0; i<len; i++) {
      stream << vec[i] << ' ';
    }
    sqlite3_result_text(ctx, stream.str().c_str(), -1, SQLITE_TRANSIENT);
  }

  int sqlite3_extension_init(sqlite3 *db, char **err, const sqlite3_api_routines *api) {
    SQLITE_EXTENSION_INIT2(api)
    // Regular Expressions
    sqlite3_create_function(db, "match", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, match, NULL, NULL);
    sqlite3_create_function(db, "search", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, search, NULL, NULL);
    sqlite3_create_function(db, "sub", 3, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sub, NULL, NULL);

    // Math
    sqlite3_create_function(db, "sin", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlsin, NULL, NULL);
    sqlite3_create_function(db, "asin", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlasin, NULL, NULL);
    sqlite3_create_function(db, "cos", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlcos, NULL, NULL);
    sqlite3_create_function(db, "acos", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlacos, NULL, NULL);
    sqlite3_create_function(db, "tan", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqltan, NULL, NULL);
    sqlite3_create_function(db, "atan", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlatan, NULL, NULL);
    sqlite3_create_function(db, "log", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqllog, NULL, NULL);
    sqlite3_create_function(db, "exp", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlexp, NULL, NULL);
    sqlite3_create_function(db, "pow", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlpow, NULL, NULL);
    sqlite3_create_function(db, "sqrt", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlsqrt, NULL, NULL);

    // Vector operations
    sqlite3_create_function(db, "vread", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlvread, NULL, NULL);
    sqlite3_create_function(db, "vshow", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlvshow, NULL, NULL);
    //sqlite3_create_function(db, "vburst", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlvburst, NULL, NULL);
    //sqlite3_create_function(db, "vcollapse", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlvcollase, NULL, NULL);
    sqlite3_create_function(db, "vzero", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlvzero, NULL, NULL);
    sqlite3_create_function(db, "vone", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlvone, NULL, NULL);
    sqlite3_create_function(db, "add", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqladd, NULL, NULL);
    sqlite3_create_function(db, "subtract", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlsubtract, NULL, NULL);
    sqlite3_create_function(db, "mult", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlmult, NULL, NULL);
    sqlite3_create_function(db, "div", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqldiv, NULL, NULL);
    sqlite3_create_function(db, "vsum", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlvsum, NULL, NULL);
    sqlite3_create_function(db, "vprod", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlvprod, NULL, NULL);
    sqlite3_create_function(db, "dot", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqldot, NULL, NULL);
    sqlite3_create_function(db, "cossim", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlcossim, NULL, NULL);
    return 0;
  }
}
