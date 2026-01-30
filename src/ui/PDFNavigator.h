#ifndef PDF_NAVIGATOR_H
#define PDF_NAVIGATOR_H

#include <QWidget>

namespace Monitor3G {

class PDFNavigator : public QWidget {
  Q_OBJECT

public:
  explicit PDFNavigator(QWidget *parent = nullptr);
};

} // namespace Monitor3G

#endif // PDF_NAVIGATOR_H
