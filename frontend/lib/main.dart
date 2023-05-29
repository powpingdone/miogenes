import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'ffi.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (_) => MioTopLevelState(),
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
  State<MioEntryPoint> createState() => _MioEntryPointState();
}

// window state
enum CurrentViewport {
  login,
}

class MioTopLevelState extends ChangeNotifier {
  CurrentViewport viewport = CurrentViewport.login;
}

class _MioEntryPointState extends State<MioEntryPoint> {
  @override
  Widget build(BuildContext context) {
    var mtls = context.watch<MioTopLevelState>();

    Widget body;
    switch (mtls.viewport) {
      case CurrentViewport.login:
        body = LoginPage();
        break;
    }

    return Scaffold(appBar: AppBar(), body: body);
  }
}

class LoginPage extends StatefulWidget {
  const LoginPage({super.key});

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  late TextEditingController _baseUrlController;

  @override
  void initState() {
    super.initState();
    _baseUrlController = TextEditingController();
  }

  @override
  void dispose() {
    super.dispose();
    _baseUrlController.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Text("Base Url:"),
        TextField(
          controller: _baseUrlController,
        )
      ],
    );
  }
}
