import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'ffi.dart';
import 'login.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatefulWidget {
  const MyApp({super.key});

  @override
  State<MyApp> createState() => _MyAppState();
}

class _MyAppState extends State<MyApp> {
  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (_) => MioTopLevel(),
      child: MaterialApp(
        title: 'Miogenes',
        theme: ThemeData(
          colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
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
}
