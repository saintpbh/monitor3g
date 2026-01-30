#include "core/DeckLinkDevice.h"
#include "ui/MainWindow.h"
#include "utils/ErrorHandler.h"
#include "utils/Logger.h"
#include <QApplication>
#include <QMessageBox>

using namespace Monitor3G;

int main(int argc, char *argv[]) {
  QApplication app(argc, argv);

  // Set application metadata
  QApplication::setApplicationName("Monitor3G");
  QApplication::setApplicationVersion("1.0.0");
  QApplication::setOrganizationName("Monitor3G");

  // Initialize logger
  Logger::instance().setLogLevel(LogLevel::DEBUG);
  Logger::instance().setLogFile("monitor3g.log");
  LOG_INFO("=== Monitor3G Starting ===");

  // Check for DeckLink devices
  QList<DeviceInfo> devices = DeckLinkDevice::enumerateDevices();
  if (devices.isEmpty()) {
    LOG_WARNING("No DeckLink devices found");
    QMessageBox::warning(
        nullptr, "No DeckLink Device",
        "No Blackmagic DeckLink devices were found.\n\n"
        "Please ensure:\n"
        "1. Your device is connected\n"
        "2. DeckLink drivers are installed\n"
        "3. The device is powered on\n\n"
        "The application will start but output will not be available.");
  } else {
    LOG_INFO(QString("Found %1 DeckLink device(s):").arg(devices.size()));
    for (const auto &device : devices) {
      LOG_INFO(QString("  - %1").arg(device.displayName));
    }
  }

  // Create and show main window
  MainWindow mainWindow;
  mainWindow.show();

  LOG_INFO("Main window displayed");

  return app.exec();
}
