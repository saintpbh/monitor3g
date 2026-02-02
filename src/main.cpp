#include "core/AppController.h"
#include "ui/qml/FrameProvider.h"
#include "ui/qml/VideoItem.h"
#include "utils/Logger.h"
#include <QApplication>
#include <QIcon>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>

using namespace Monitor3G;

int main(int argc, char *argv[]) {
  // Use QApplication (not QGuiApplication) because we link against Widgets
  // and might use dialogs or existing Widget-based logic internally.
  QApplication app(argc, argv);

  // Set application metadata
  QApplication::setApplicationName("Monitor3G");
  QApplication::setApplicationVersion("1.1.0");
  QApplication::setOrganizationName("Monitor3G");

  QQuickStyle::setStyle("Fusion");

  // Initialize logger
  Logger::instance().setLogLevel(LogLevel::DEBUG);
  Logger::instance().setLogFile("monitor3g.log");
  LOG_INFO("=== Monitor3G (QML) Starting ===");

  // Register QML Types
  qmlRegisterType<VideoItem>("Monitor3G", 1, 0, "VideoItem");

  // Initialize Backend Controller
  AppController backend;
  backend.initialize();

  // Create QML Engine
  QQmlApplicationEngine engine;

  // Expose backend to QML
  engine.rootContext()->setContextProperty("backend", &backend);

  // Expose FrameProvider singleton to QML
  engine.rootContext()->setContextProperty("FrameProvider",
                                           &FrameProvider::instance());

  // Load main.qml
  // Note: In production, this should be a resource path (qrc:/).
  // For development iteration, we use the absolute path.
  const QUrl url(QStringLiteral(
      "file:///Users/bongpark/Library/CloudStorage/"
      "OneDrive-한국기독교장로회총회유지재단/0.박봉환개인문서폴더/앱개발/"
      "Monitor3G/src/ui/qml/main.qml"));

  QObject::connect(
      &engine, &QQmlApplicationEngine::objectCreated, &app,
      [url](QObject *obj, const QUrl &objUrl) {
        if (!obj && url == objUrl)
          QCoreApplication::exit(-1);
      },
      Qt::QueuedConnection);

  engine.load(url);

  return app.exec();
}
