#include "SourcePanel.h"
#include <QFileDialog>
#include <QLabel>
#include <QVBoxLayout>

namespace Monitor3G {

SourcePanel::SourcePanel(QWidget *parent) : QWidget(parent) { setupUI(); }

void SourcePanel::setupUI() {
  setAcceptDrops(true);
  QVBoxLayout *layout = new QVBoxLayout(this);
  layout->setContentsMargins(8, 8, 8, 8);
  layout->setSpacing(8);

  // Title
  QLabel *title = new QLabel(tr("Sources"), this);
  QFont titleFont = title->font();
  titleFont.setPointSize(12);
  titleFont.setBold(true);
  title->setFont(titleFont);
  layout->addWidget(title);

  // Source list
  m_sourceList = new QListWidget(this);
  m_sourceList->setAlternatingRowColors(true);
  layout->addWidget(m_sourceList, 1);

  // Add source buttons
  m_addVideoBtn = new QPushButton(tr("📹 Video File"), this);
  connect(m_addVideoBtn, &QPushButton::clicked, this,
          &SourcePanel::onAddVideoClicked);
  layout->addWidget(m_addVideoBtn);

  m_addImageBtn = new QPushButton(tr("🖼️  Image"), this);
  connect(m_addImageBtn, &QPushButton::clicked, this,
          &SourcePanel::onAddImageClicked);
  layout->addWidget(m_addImageBtn);

  m_addPDFBtn = new QPushButton(tr("📄 PDF"), this);
  connect(m_addPDFBtn, &QPushButton::clicked, this,
          &SourcePanel::onAddPDFClicked);
  layout->addWidget(m_addPDFBtn);

  m_addScreenBtn = new QPushButton(tr("🖥️  Screen Capture"), this);
  connect(m_addScreenBtn, &QPushButton::clicked, this,
          &SourcePanel::onAddScreenCaptureClicked);
  layout->addWidget(m_addScreenBtn);

  m_addCameraBtn = new QPushButton(tr("📷 Live Camera"), this);
  connect(m_addCameraBtn, &QPushButton::clicked, this,
          &SourcePanel::onAddLiveCameraClicked);
  layout->addWidget(m_addCameraBtn);
}

void SourcePanel::onAddVideoClicked() {
  QString fileName = QFileDialog::getOpenFileName(
      this, tr("Select Video File"), QString(),
      tr("Video Files (*.mp4 *.mov *.avi *.mkv *.m4v);;All Files (*)"));

  if (!fileName.isEmpty()) {
    m_sourceList->addItem(tr("Video: %1").arg(QFileInfo(fileName).fileName()));
    emit sourceAdded(fileName, "video");
  }
}

void SourcePanel::dragEnterEvent(QDragEnterEvent *event) {
  if (event->mimeData()->hasUrls()) {
    event->acceptProposedAction();
  }
}

void SourcePanel::dropEvent(QDropEvent *event) {
  const QMimeData *mimeData = event->mimeData();
  if (mimeData->hasUrls()) {
    QList<QUrl> urlList = mimeData->urls();
    for (const QUrl &url : urlList) {
      QString path = url.toLocalFile();
      QFileInfo info(path);
      QString suffix = info.suffix().toLower();

      if (suffix == "mp4" || suffix == "mov" || suffix == "avi" ||
          suffix == "mkv" || suffix == "m4v") {
        m_sourceList->addItem(tr("Video: %1").arg(info.fileName()));
        emit sourceAdded(path, "video");
      } else if (suffix == "png" || suffix == "jpg" || suffix == "jpeg" ||
                 suffix == "bmp") {
        m_sourceList->addItem(tr("Image: %1").arg(info.fileName()));
        emit sourceAdded(path, "image");
      } else if (suffix == "pdf") {
        m_sourceList->addItem(tr("PDF: %1").arg(info.fileName()));
        emit sourceAdded(path, "pdf");
      }
    }
    event->acceptProposedAction();
  }
}

void SourcePanel::onAddImageClicked() {
  QString fileName = QFileDialog::getOpenFileName(
      this, tr("Select Image File"), QString(),
      tr("Image Files (*.png *.jpg *.jpeg *.bmp *.tiff);;All Files (*)"));

  if (!fileName.isEmpty()) {
    m_sourceList->addItem(tr("Image: %1").arg(QFileInfo(fileName).fileName()));
  }
}

void SourcePanel::onAddPDFClicked() {
  QString fileName =
      QFileDialog::getOpenFileName(this, tr("Select PDF File"), QString(),
                                   tr("PDF Files (*.pdf);;All Files (*)"));

  if (!fileName.isEmpty()) {
    m_sourceList->addItem(tr("PDF: %1").arg(QFileInfo(fileName).fileName()));
  }
}

void SourcePanel::onAddScreenCaptureClicked() {
  // TODO: Implement screen capture dialog
  m_sourceList->addItem(tr("Screen Capture"));
}

void SourcePanel::onAddLiveCameraClicked() {
  // TODO: Implement camera selection dialog
  m_sourceList->addItem(tr("Live Camera"));
}

} // namespace Monitor3G
