import 'dart:ffi';

import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';

import 'bridge_generated.dart';

// Re-export the bridge so it is only necessary to import this file.
export 'bridge_generated.dart';
import 'dart:io' as io;

const _base = 'mio_glue';

// On MacOS, the dynamic library is not bundled with the binary,
// but rather directly **linked** against the binary.
final _dylib = io.Platform.isWindows ? '$_base.dll' : 'lib$_base.so';

final MioGlueImpl api = MioGlueImpl(io.Platform.isIOS || io.Platform.isMacOS
    ? DynamicLibrary.executable()
    : DynamicLibrary.open(_dylib));

String extractMsg(dynamic error) {
  // If you're reading this and you got a PANIC_ERROR or another dart exception, please report.
  // Normal code is definitely not supposted to panic or throw dart execeptions.
  // As Tim Peters said in The Zen of Python (a bit extrapolated):
  //    "Readability counts."
  //    "Errors should never pass silently."
  //    "Unless explicitly silenced."
  // This means that all errors should be handled and the user should
  // have a understanding of what happened,
  //
  // but i'm not a fuckin perfectionist.
  // https://github.com/powpingdone/miogenes/issues
  if (error.runtimeType == FfiException) {
    var err = error as FfiException;
    if (err.code == "RESULT_ERROR") {
      // return nice, pretty message. thanks `anyhow`.
      return err.message;
    }
  }
  // normal code should not panic, but if it does,
  // it'll be "fucked up" and the error message should show everything.
  // or this some other exception that I didn't handle.
  // whatever. if the error reaches here this is definitely a bug.
  return error.toString();
}
