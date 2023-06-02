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
enum CurrentViewport {
  login,
}

class MioTopLevel with ChangeNotifier {
  CurrentViewport viewport = CurrentViewport.login;
  final MioClient _mioInternal = api.newMioClient();
  MioClient get mioClient => _mioInternal;
}

class _MioTopLevel extends State<MioEntryPoint> {
  @override
  Widget build(BuildContext context) {
    var mtl = context.watch<MioTopLevel>();

    Widget body;
    switch (mtl.viewport) {
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
  late TextEditingController _baseUrlController,
      _usernameController,
      _passwordController;
  Future<void>? isValidUrl;

  @override
  void initState() {
    super.initState();
    _baseUrlController = TextEditingController();
    _usernameController = TextEditingController();
    _passwordController = TextEditingController();
  }

  @override
  void dispose() {
    super.dispose();
    _baseUrlController.dispose();
    _usernameController.dispose();
    _passwordController.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final mtl = Provider.of<MioTopLevel>(context);
    var mioState = mtl.mioClient;

    return Column(
      children: [
        const Text("Base Url:"),
        TextField(
          controller: _baseUrlController,
          onSubmitted: (url) {
            setState(() {
              isValidUrl = mioState.testSetUrl(url: url);
            });
          },
        ),
        // Username and Password
        FutureBuilder(
          future: isValidUrl,
          builder: (context, snapshot) {
            if (snapshot.hasData) {
              // build ui
              return Text("Valid url!");
            } else if (snapshot.hasError) {
              return Text("Invalid url: ${snapshot.error.toString()}");
            } else if (isValidUrl != null) {
              return Text("Checking server...");
            } else {
              // do nothing
              return Container();
            }
          },
        )
      ],
    );
  }
}
