import 'dart:collection';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:frontend/mio_albums_page.dart';
import 'package:frontend/mio_player_page.dart';
import 'package:frontend/mio_upload_page.dart';
import 'package:provider/provider.dart';
import 'package:uuid/uuid.dart';

// Holder for providers
class MainPageProviders extends StatefulWidget {
  const MainPageProviders({super.key});

  @override
  State<StatefulWidget> createState() => _MainPageProvidersState();
}

class _MainPageProvidersState extends State<MainPageProviders> {
  static const List<String> _initPages = [
    "Player",
    "Albums",
    "Upload",
  ];

  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        ChangeNotifierProvider(
          create: (_) => PageAt(_initPages),
        )
      ],
      child: const MainPageScaffold(),
    );
  }
}

// Current main page
class PageAt with ChangeNotifier, DiagnosticableTreeMixin {
  late Set<String> _pages;
  late List<String> _pageOrder;
  PageAt(List<String> order) {
    _pageOrder = List.unmodifiable(order);
    _pages = _pageOrder.toSet();
  }
  String _page = "Player";

  // what page are we on
  String get page => _page;
  set page(String page) {
    if (_pages.contains(page)) {
      _page = page;
    } else {
      _page = "Player";
    }
    notifyListeners();
  }

  // total pages set
  List<String> get pageOrder => List.unmodifiable(_pageOrder);
  Set<String> get pages => Set.unmodifiable(_pages);
  void reorderPages(List<String> newList) {
    _pageOrder = List.unmodifiable(newList);
    _pages = _pageOrder.toSet();
    if (!_pages.contains(_page)) {
      // the page got removed
      _page = "Player";
    }
    notifyListeners();
  }

  /// devtools
  @override
  void debugFillProperties(DiagnosticPropertiesBuilder properties) {
    super.debugFillProperties(properties);
    properties.add(StringProperty("page", _page));
    properties.add(DiagnosticsProperty("pages", _pageOrder));
  }
}

class PlayerIntState with ChangeNotifier, DiagnosticableTreeMixin {
  Queue<Uuid> _queue = ListQueue(16);
  int? _playingId = 0;
  //final _player = AudioPlayer();

  Future<void> play() async {}

  Future<void> pause() async {}

  Future<void> next() async {}

  Future<void> prev() async {}

  Future<void> clear() async {}

  Future<void> enqueue(Uuid uuid) async {}

  /// devtools
  @override
  void debugFillProperties(DiagnosticPropertiesBuilder properties) {
    super.debugFillProperties(properties);
  }
}

// Scaffold holder
class MainPageScaffold extends StatefulWidget {
  const MainPageScaffold({super.key});

  @override
  State<MainPageScaffold> createState() => _MainPageScaffoldState();
}

class _MainPageScaffoldState extends State<MainPageScaffold> {
  @override
  Widget build(BuildContext context) {
    final pageAt = context.watch<PageAt>();

    return Scaffold(
      appBar: AppBar(
        // A shortcut to get back to the main player
        title: TextButton(
            onPressed: () => context.read<PageAt>().page = "Player",
            child: Text(pageAt.page)),
      ),
      drawer: Drawer(
        child: SliverList.builder(itemBuilder: (context, i) {
          return ListTile(
            title: Text(pageAt.pageOrder[i]),
            onTap: () => setState(
              () => context.read<PageAt>().page = pageAt.pageOrder[i],
            ),
          );
        }),
      ),
      body: const MioView(),
    );
  }
}

// The inner switcher for the external pages
class MioView extends StatefulWidget {
  const MioView({super.key});

  @override
  State<MioView> createState() => _MioViewState();
}

class _MioViewState extends State<MioView> {
  final Map<String, Future<Widget>> _pageMap = {};

  @override
  Widget build(BuildContext context) {
    // we depend on this
    final pageAt = context.watch<PageAt>();
    // add new pages
    pageAt.pages
        .difference(_pageMap.keys.toSet())
        .forEach((newPage) => _pageMap[newPage] = _buildPageMapItem(newPage));
    // remove pages not in self
    _pageMap.keys.toSet().difference(pageAt.pages).forEach(_pageMap.remove);
    return FutureBuilder(
        future: _pageMap[pageAt.page],
        builder: (context, snapshot) {
          if (snapshot.hasData) {
            return snapshot.data!;
          } else if (snapshot.hasError) {
            return MioFullPageError(snapshot.error.toString());
          } else {
            return const SizedBox.shrink(
                child: CircularProgressIndicator.adaptive());
          }
        });
  }

  // Map a "name" to the actual widget to build in the background
  Future<Widget> _buildPageMapItem(String name) {
    // this will be expanded upon in the future for building custom pages at runtime
    return compute(
        (name) => switch (name) {
              "Player" => const MioPlayerPage(),
              "Albums" => const MioAlbumsPage(),
              "Upload" => const MioUploadPage(),
              _ => MioFullPageError(
                  "_buildPageMapItem got '$name' instead of expected input"),
            },
        name);
  }
}

/// Class for page generation errors. Only used as a widget for displaying such
class MioFullPageError extends StatelessWidget {
  const MioFullPageError(
    this.errorString, {
    super.key,
  });
  final String errorString;

  @override
  Widget build(BuildContext context) {
    return Text(errorString);
  }
}
