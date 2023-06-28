import 'package:flutter/material.dart';
import 'package:frontend/mainpage/mainpage.dart';
import 'package:frontend/signup.dart';
import 'package:provider/provider.dart';
import 'ffi.dart';
import 'login.dart';

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
  final MioClient _mioInternal = api.newMioClient();
  MioClient get mioClient => _mioInternal;
}

class _MioTopLevel extends State<MioEntryPoint> {
  @override
  Widget build(BuildContext context) {
    return Scaffold(
        appBar: AppBar(),
        body: Navigator(
          initialRoute: "login",
          onGenerateRoute: (settings) {
            WidgetBuilder builder;
            switch (settings.name) {
              case "login":
                builder = (context) => const LoginBaseUrl();
                break;
              case "signup":
                builder = (context) => const SignupPage();
                break;
              case "mainpage":
                builder = (context) => const MainNav();
                break;
              default:
                throw UnimplementedError(
                    "No such route ${settings.name} exists");
            }
            return MaterialPageRoute(builder: builder, settings: settings);
          },
        ));
  }
}
