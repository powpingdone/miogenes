package com.example.frontend

import io.flutter.embedding.android.FlutterActivity

// https://github.com/dart-lang/sdk/issues/46027
class MainActivity : FlutterActivity() {
    init {
        System.loadLibrary("mio_glue")
    }
}
