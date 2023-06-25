import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'ffi.dart';
import 'login.dart';
import 'signup.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

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
        home: const MioEntryPoint(),
      ),
    );
  }
}

class MioEntryPoint extends StatefulWidget {
  const MioEntryPoint({super.key});

  @override
  State<MioEntryPoint> createState() => _MioTopLevel();
}

// window state
enum CurrentViewport { login, signup, mainpage }

class MioTopLevel with ChangeNotifier {
  CurrentViewport _viewport = CurrentViewport.login;
  CurrentViewport get viewport => _viewport;
  set viewport(CurrentViewport viewport) {
    _viewport = viewport;
    notifyListeners();
  }
  final MioClient _mioInternal = api.newMioClient();
  MioClient get mioClient => _mioInternal;
}

class _MioTopLevel extends State<MioEntryPoint>  {
  @override
  Widget build(BuildContext context) {
    var mtl = context.watch<MioTopLevel>();

    Widget body;
    switch (mtl.viewport) {
      case CurrentViewport.login:
        body = const LoginBaseUrl();
        break;
      case CurrentViewport.signup:
        body = const SignupPage();
        break;
      case CurrentViewport.mainpage:
        body = Container();
        break;
    }

    return Scaffold(appBar: AppBar(), body: body);
  }
}
