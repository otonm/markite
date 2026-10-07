// Remote file I/O via KIO, called from kde/src/kio.rs.
// Why not plain file I/O: Dolphin hands non-KIO apps a kio-fuse path, and kio-fuse's write
// path (open O_TRUNC + write) is unreliable — kio-fuse issue #10 (LibreOffice, EIO/EPERM).
// Going through KIO directly bypasses the fuse mount.
// Markdown files are small, so a nested event loop keeps these wrappers synchronous.
#include <QEventLoop>
#include <QUrl>
#include <KJob>
#include <kio/storedtransferjob.h>
#include <cstdlib>
#include <cstring>
#include <cstdio>

// Debug traces: compiled in only with `--features trace` (build.rs defines MARKITE_TRACE); absent from release builds.
#ifdef MARKITE_TRACE
#define TRACE(...) do { fprintf(stderr, "[trace kio_shim] " __VA_ARGS__); fputc('\n', stderr); } while (0)
#else
#define TRACE(...) ((void)0)
#endif

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
    TRACE("finish: running a nested event loop until the KIO job finishes");
    loop.exec();
    if (job->error() != KJob::NoError) {
        TRACE("finish: job failed (code %d): %s", job->error(), job->errorString().toUtf8().constData());
        *err = dup_bytes(job->errorString().toUtf8());
        return 1;
    }
    return 0;
}

extern "C" int markite_kio_read(const char *url, char **data, long long *len, char **err)
{
    *data = nullptr;
    *err = nullptr;
    TRACE("read: storedGet %s", url);
    auto *job = KIO::storedGet(QUrl(QString::fromUtf8(url)));
    if (int rc = finish(job, err); rc != 0)
        return rc;
    const QByteArray bytes = job->data();
    TRACE("read: ok, %lld bytes", (long long)bytes.size());
    *len = bytes.size();
    *data = dup_bytes(bytes);
    return 0;
}

extern "C" int markite_kio_write(const char *url, const char *data, long long len, char **err)
{
    *err = nullptr;
    TRACE("write: storedPut %lld bytes to %s (Overwrite, no progress UI)", len, url);
    auto *job = KIO::storedPut(QByteArray(data, int(len)), QUrl(QString::fromUtf8(url)),
                               -1, KIO::Overwrite | KIO::HideProgressInfo);
    return finish(job, err);
}
