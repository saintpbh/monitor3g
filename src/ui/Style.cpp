#include "Style.h"
#include <QFile>
#include <QTextStream>
#include <QDebug>

namespace Monitor3G {

namespace {
    QString readThemeFromFile(const QString& path) {
        QFile file(path);
        if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
            qWarning() << "Could not open theme file:" << path;
            return QString();
        }
        QTextStream in(&file);
        return in.readAll();
    }
}

QString Style::getDarkTheme() {
  // Better to use Qt Resource System (.qrc) to bundle assets.
  return readThemeFromFile(":/styles/dark.qss");
}

QString Style::getLightTheme() {
  return readThemeFromFile(":/styles/light.qss");
}

} // namespace Monitor3G
