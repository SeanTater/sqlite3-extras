#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <string>
#include <sstream>
#include <boost/regex.hpp>
#include <sqlite3ext.h>
#include <cmath>
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

  /*****************************************************************************
   * SQLite-visible functions, aggregates and wrappers (all start with sql_)
   */

   void sql_sin(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], sin);
   }
   void sql_asin(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], asin);
   }
   void sql_cos(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], cos);
   }
   void sql_acos(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], acos);
   }
   void sql_tan(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], tan);
   }
   void sql_atan(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], atan);
   }
   void sql_log(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], log);
   }
   void sql_exp(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], exp);
   }
   void sql_pow(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vbinop(ctx, argv, pow);
   }
   void sql_sqrt(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
     vunop(ctx, argv[0], sqrt);
   }

  // Create a new zero vector
  void sql_vzero(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    int len = sqlite3_value_int(argv[0]);

    real result_vec[len];
    for (int i=0; i<len; i++) {
      result_vec[i] = 0;
    }
    sqlite3_result_blob(ctx, result_vec, len*sizeof(real), SQLITE_TRANSIENT);
  }

  // Create a new one vector
  void sql_vone(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    int len = sqlite3_value_int(argv[0]);

    real result_vec[len];
    for (int i=0; i<len; i++) {
      result_vec[i] = 1;
    }
    sqlite3_result_blob(ctx, result_vec, len*sizeof(real), SQLITE_TRANSIENT);
  }

  // Add two vectors
  void sql_add(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    vbinop(ctx, argv, [](real a, real b){ return a + b; });
  }
  // Subtract the second vector from the first
  void sql_subtract(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    vbinop(ctx, argv, [](real a, real b){ return a - b; });
  }
  // Multiply two vectors
  void sql_mult(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    vbinop(ctx, argv, [](real a, real b){ return a * b; });
  }
  // Divide two vectors
  void sql_div(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    vbinop(ctx, argv, [](real a, real b){ return a / b; });
  }

  // Complain to SQLite if we don't get the vectors we want (saves code later)
  bool must_be_vectors(
      const char *name,
      sqlite3_context *ctx,
      int argc,
      sqlite3_value **argv) {
    for (int i=0; i<argc; i++) {
      if (sqlite3_value_type(argv[i]) != SQLITE_BLOB) {
        char buffer[100];
        snprintf(buffer, 100, "Wrong datatype supplied. %s requires %d vectors.", name, argc);
        sqlite3_result_error(ctx, buffer, -1);
        return false;
      }
    }
    return true;
  }

  // Compute the sum of the elements of a vector: It's not the most numerically
  // stable way though.
  void sql_vsum(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    if (!must_be_vectors("vsum", ctx, 1, argv)) return;
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
  void sql_vprod(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    if (!must_be_vectors("vprod", ctx, 1, argv)) return;
    int len = sqlite3_value_bytes(argv[0]) / sizeof(real);
    real *vec = (real*) sqlite3_value_blob(argv[0]);
    real end = 0.0;
    for (int i=0; i<len; i++) {
      end *= vec[i];
    }
    sqlite3_result_double(ctx, end);
  }

  // Compute the dot product: It's not the most numerically stable way though.
  void sql_dot(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    if (!must_be_vectors("dot", ctx, 2, argv)) return;
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
  void sql_cossim(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    if (!must_be_vectors("cossim", ctx, 2, argv)) return;
    int len = fmin(sqlite3_value_bytes(argv[0]), sqlite3_value_bytes(argv[1]))
              / sizeof(real);

    real *a = (real*) sqlite3_value_blob(argv[0]);
    real *b = (real*) sqlite3_value_blob(argv[1]);

    real asq = 0.0;  for (int i=0; i<len; i++) asq  += a[i] * a[i];
    real bsq = 0.0;  for (int i=0; i<len; i++) bsq  += b[i] * b[i];
    real absq = 0.0; for (int i=0; i<len; i++) absq += a[i] * b[i];

    sqlite3_result_double(ctx, absq / (sqrt(asq) * sqrt(bsq)));
  }

  typedef struct FatBuffer {
    int len; // Length of the content array (in bytes)
    int count; // Useful for accumulators.
    real content[1]; // The actual data (probably longer than 1, btw)
  } FatBuffer;
  int wrapped_size(int len_reals) {
    return (2*sizeof(int)) + (len_reals*sizeof(real));
  };



  // Compute the sum of many vectors (as part of an aggregate)
  void sql_vsum_aggregate_step(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    if (!must_be_vectors("vsum_aggregate", ctx, 1, argv)) return;
    /// --- The only hard part here is allocating the accumulator. ---
    // First decide how long we'd like it to be *in units*. Optimally it should
    // be the same size as the vector we're reading right now.
    int target_len = sqlite3_value_bytes(argv[0]) / sizeof(real);
    if (target_len == 0) return; // Do nothing for empty vectors.
    // Maybe allocate it (or it may return an existing buffer of any length
    // or it may return NULL.)
    FatBuffer *accum = (FatBuffer*) sqlite3_aggregate_context(ctx, wrapped_size(target_len));
    if (accum == NULL) {sqlite3_result_error_nomem(ctx); return;} // die.
    // Find out how long the buffer really is, *in bytes*
    // If it's new, it's 0'd out so the length will be 0 and we know we got the
    // size we asked for. Otherwise use the length we asked for.
    if (accum->len == 0) accum->len = target_len;
    // Now we should iterate the lesser of the two lengths (in units of reals)
    int iter_len = fmin(accum->len, target_len);
    real *vec = (real*) sqlite3_value_blob(argv[0]);
    for (int i=0; i<iter_len; i++) {
      accum->content[i] += vec[i];
    }
    accum->count++;
  }

  // Compute the sum of many vectors (as part of an aggregate) (final part)
  void sql_vsum_aggregate_final(sqlite3_context *ctx) {
    /// --- The only hard part here is allocating the accumulator. ---
    // Maybe allocate it (or it may return an existing buffer of any length
    // or it may return NULL.)
    FatBuffer *accum = (FatBuffer*) sqlite3_aggregate_context(ctx, 0);
    if (accum == NULL) {
      // I'm not sure if a zero-length blob or NULL is better.
      // I'm choosing this right now merely because of my dislike for NULL.
      sqlite3_result_zeroblob(ctx, 0);
    } else {
      sqlite3_result_blob(ctx, accum->content, accum->len, SQLITE_TRANSIENT);
    }
  }
  // Compute the average of many vectors (as part of an aggregate) (final part)
  void sql_vavg_aggregate_final(sqlite3_context *ctx) {
    /// --- The only hard part here is allocating the accumulator. ---
    // Maybe allocate it (or it may return an existing buffer of any length
    // or it may return NULL.)
    FatBuffer *accum = (FatBuffer*) sqlite3_aggregate_context(ctx, 0);
    if (accum == NULL) {
      // I'm not sure if a zero-length blob or NULL is better.
      // I'm choosing this right now merely because of my dislike for NULL.
      sqlite3_result_zeroblob(ctx, 0);
    } else {
      for (int i=0; i<accum->len; i++) accum->content[i] /= accum->count;
      sqlite3_result_blob(ctx, accum->content, accum->len, SQLITE_TRANSIENT);
    }
  }

  // Read a vector from a space separated string
  void sql_vread(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
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
  void sql_vshow(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
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
    sqlite3_create_function(db, "match",  2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, match, NULL, NULL);
    sqlite3_create_function(db, "search", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, search, NULL, NULL);
    sqlite3_create_function(db, "sub",    3, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sub, NULL, NULL);

    // Math
    sqlite3_create_function(db, "sin",    1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_sin, NULL, NULL);
    sqlite3_create_function(db, "asin",   1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_asin, NULL, NULL);
    sqlite3_create_function(db, "cos",    1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_cos, NULL, NULL);
    sqlite3_create_function(db, "acos",   1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_acos, NULL, NULL);
    sqlite3_create_function(db, "tan",    1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_tan, NULL, NULL);
    sqlite3_create_function(db, "atan",   1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_atan, NULL, NULL);
    sqlite3_create_function(db, "log",    1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_log, NULL, NULL);
    sqlite3_create_function(db, "exp",    1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_exp, NULL, NULL);
    sqlite3_create_function(db, "pow",    2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_pow, NULL, NULL);
    sqlite3_create_function(db, "sqrt",   1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_sqrt, NULL, NULL);

    // Vector operations
    sqlite3_create_function(db, "vread", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_vread, NULL, NULL);
    sqlite3_create_function(db, "vshow", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_vshow, NULL, NULL);
    //sqlite3_create_function(db, "vburst", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlvburst, NULL, NULL);
    //sqlite3_create_function(db, "vcollapse", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sqlvcollase, NULL, NULL);
    sqlite3_create_function(db, "vzero",  1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_vzero, NULL, NULL);
    sqlite3_create_function(db, "vone",   1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_vone, NULL, NULL);
    sqlite3_create_function(db, "add",    2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_add, NULL, NULL);
    sqlite3_create_function(db, "subtract", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_subtract, NULL, NULL);
    sqlite3_create_function(db, "mult",   2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_mult, NULL, NULL);
    sqlite3_create_function(db, "div",    2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_div, NULL, NULL);
    sqlite3_create_function(db, "vsum",   1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_vsum, NULL, NULL);
    sqlite3_create_function(db, "vprod",  1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_vprod, NULL, NULL);
    sqlite3_create_function(db, "dot",    2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_dot, NULL, NULL);
    sqlite3_create_function(db, "cossim", 2, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, sql_cossim, NULL, NULL);

    // Aggregate functions
    sqlite3_create_function(db, "vsum_aggregate", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, NULL, sql_vsum_aggregate_step, sql_vsum_aggregate_final);
    sqlite3_create_function(db, "vavg_aggregate", 1, SQLITE_UTF8 | SQLITE_DETERMINISTIC, NULL, NULL, sql_vsum_aggregate_step, sql_vavg_aggregate_final);
    return 0;
  }
}
