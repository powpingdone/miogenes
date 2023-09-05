package xyz.powpingdone.miogenes

import com.ryanheise.audioservice.AudioServiceActivity

import io.flutter.embedding.android.FlutterActivity


// https://github.com/dart-lang/sdk/issues/46027
class MainActivity : AudioServiceActivity() {
    init {
        System.loadLibrary("mio_glue")
    }
}
