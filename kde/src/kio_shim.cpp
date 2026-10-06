// Remote file I/O via KIO, called from kde/src/kio.rs.
// Why not plain file I/O: Dolphin hands non-KIO apps a kio-fuse path, and kio-fuse's write
// path (open O_TRUNC + write) is unreliable — kio-fuse issue #10 (LibreOffice, EIO/EPERM).
// Going through KIO directly bypasses the fuse mount, like Kate does.
// Markdown files are small, so a nested event loop keeps these wrappers synchronous.
#include <QEventLoop>
#include <QUrl>
#include <KJob>
#include <kio/storedtransferjob.h>
#include <cstdlib>
#include <cstring>

extern "C" void markite_kio_free(char *p) { free(p); }

static char *dup_bytes(const QByteArray &bytes)
{
    char *out = (char *)malloc(bytes.size() + 1);
    memcpy(out, bytes.constData(), bytes.size() + 1);
    return out;
}

static int finish(KJob *job, char **err)
{
    QEventLoop loop;
    QObject::connect(job, &KJob::finished, &loop, &QEventLoop::quit);
    loop.exec();
    if (job->error() != KJob::NoError) {
        *err = dup_bytes(job->errorString().toUtf8());
        return 1;
    }
    return 0;
}

extern "C" int markite_kio_read(const char *url, char **data, long long *len, char **err)
{
    *data = nullptr;
    *err = nullptr;
    auto *job = KIO::storedGet(QUrl(QString::fromUtf8(url)));
    if (int rc = finish(job, err); rc != 0)
        return rc;
    const QByteArray bytes = job->data();
    *len = bytes.size();
    *data = dup_bytes(bytes);
    return 0;
}

extern "C" int markite_kio_write(const char *url, const char *data, long long len, char **err)
{
    *err = nullptr;
    auto *job = KIO::storedPut(QByteArray(data, int(len)), QUrl(QString::fromUtf8(url)),
                               -1, KIO::Overwrite | KIO::HideProgressInfo);
    return finish(job, err);
}
