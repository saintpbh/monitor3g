#ifndef APPCONTROLLER_H
#define APPCONTROLLER_H

#include <QImage>
#include <QObject>
#include <QStringList>
#include <QTimer>
#include <memory>
#include <vector>

namespace Monitor3G {
class DeckLinkDevice;
class DeckLinkOutput;
class AbstractSource;
class TestPatternSource;
class VideoFileSource;
} // namespace Monitor3G

namespace Monitor3G {

class AppController : public QObject {
  Q_OBJECT
  Q_PROPERTY(QString deviceStatus READ deviceStatus NOTIFY deviceStatusChanged)
  Q_PROPERTY(
      bool isDeviceConnected READ isDeviceConnected NOTIFY deviceStatusChanged)
  Q_PROPERTY(QStringList sourceList READ sourceList NOTIFY sourceListChanged)
  // Playback Control Properties
  Q_PROPERTY(bool isPlaying READ isPlaying NOTIFY isPlayingChanged)
  Q_PROPERTY(qint64 position READ position NOTIFY positionChanged)
  Q_PROPERTY(qint64 duration READ duration NOTIFY durationChanged)
  Q_PROPERTY(int sourceType READ sourceType NOTIFY sourceTypeChanged)

public:
  explicit AppController(QObject *parent = nullptr);
  ~AppController();

  void initialize();

  QString deviceStatus() const;
  bool isDeviceConnected() const;
  QStringList sourceList() const;

  bool isPlaying() const;
  qint64 position() const;
  qint64 duration() const;
  int sourceType() const;

public slots:
  void requestAddFile();
  void selectSource(int index);
  void startOutput();
  void stopOutput();

  // Playback Slots
  void play();
  void pause();
  void seek(qint64 position);

  // Preview/Program Slots
  void takePreview();

signals:
  void deviceStatusChanged();
  void sourceListChanged();
  void isPlayingChanged();
  void positionChanged();
  void durationChanged();
  void sourceTypeChanged();

private:
  void updatePreview();
  void onSourceAdded(const QString &name);

  std::unique_ptr<DeckLinkDevice> m_device;
  DeckLinkOutput *m_output;
  TestPatternSource *m_testPatternSource;

  // Preview/Program architecture
  AbstractSource *m_previewSource;     // Selected for preview monitor
  AbstractSource *m_programSource;     // Currently on program/output
  AbstractSource *m_currentMainSource; // Deprecated, will phase out

  QList<AbstractSource *> m_sources;
  QStringList m_sourceNames;

  QTimer *m_timer;
  QString m_deviceStatusMsg;
  bool m_isDeviceConnected;
};

} // namespace Monitor3G

#endif // APPCONTROLLER_H
