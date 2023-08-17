import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'ffi.dart';
import 'login.dart';

void main() {
  runApp(const MiogenesApp());
}

class MiogenesApp extends StatefulWidget {
  const MiogenesApp({super.key});

  @override
  State<MiogenesApp> createState() => _MiogenesAppState();
}

class _MiogenesAppState extends State<MiogenesApp> {
  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (_) => MioTopLevel(),
      child: MaterialApp(
        title: 'Miogenes',
        theme: ThemeData(
          colorScheme: ColorScheme.fromSeed(seedColor: Colors.lightGreen),
          useMaterial3: true,
        ),
        home: const LoginBaseUrl(),
      ),
    );
  }
}

// window state
class MioTopLevel with ChangeNotifier {
  final MioClient _mioInternal = api.newMioClient();
  MioClient get mioClient => _mioInternal;
  MioTopLevel() {
    api.initSelf();
  }
}
