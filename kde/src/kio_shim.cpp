// Remote file I/O via KIO, called from kde/src/kio.rs.
// Why not plain file I/O: Dolphin hands non-KIO apps a kio-fuse path, and kio-fuse's write
// path (open O_TRUNC + write) is unreliable — kio-fuse issue #10 (LibreOffice, EIO/EPERM).
// Going through KIO directly bypasses the fuse mount.
// Markdown files are small, so a nested event loop keeps these wrappers synchronous.
// Known limitation: the nested loop also dispatches other app events (e.g. QML) while a transfer runs; the UI
// should not edit the document meanwhile, and no input filtering is applied because KIO may show its own dialogs.
#include <QByteArray>
#include <QEventLoop>
#include <QUrl>
#include <KJob>
#include <kio/storedtransferjob.h>
#include <cstdio>
#include <cstdlib>
#include <cstring>

// Debug traces: compiled in only with `--features trace` (build.rs defines MARKITE_TRACE); absent from release builds.
#ifdef MARKITE_TRACE
#define TRACE(...) do { std::fprintf(stderr, "[trace kio_shim] " __VA_ARGS__); std::fputc('\n', stderr); } while (0)
#else
#define TRACE(...) ((void)0)
#endif

// The URL without its password, safe to log.
[[maybe_unused]] static QByteArray shown(const QUrl &url)
{
    return url.toDisplayString(QUrl::RemovePassword).toUtf8();
}

extern "C" void markite_kio_free(char *p) { std::free(p); }

// malloc'd copy of `bytes` plus a NUL terminator, owned by the caller (Rust frees it with markite_kio_free).
// nullptr if out of memory.
static char *dup_bytes(const QByteArray &bytes)
{
    const auto size = static_cast<size_t>(bytes.size());
    auto *out = static_cast<char *>(std::malloc(size + 1));
    if (out) {
        std::memcpy(out, bytes.constData(), size);
        out[size] = '\0';
    }
    return out;
}

// Reports `message` through `err` (nullptr on out-of-memory; Rust then says "unknown KIO error") and returns 1.
static int fail(char **err, const QByteArray &message)
{
    *err = dup_bytes(message);
    return 1;
}

static int finish(KJob *job, char **err)
{
    QEventLoop loop;
    bool finished = false;
    QObject::connect(job, &KJob::finished, &loop, [&] {
        finished = true;
        loop.quit();
    });
    TRACE("finish: running a nested event loop until the KIO job finishes");
    loop.exec();
    if (!finished) { // QCoreApplication::quit() (window closed) stopped the loop, not the job
        TRACE("finish: interrupted, killing the job");
        job->kill(KJob::Quietly); // also deletes the job
        return fail(err, "interrupted");
    }
    if (job->error() != KJob::NoError) {
        TRACE("finish: job failed (code %d): %s", job->error(), job->errorString().toUtf8().constData());
        return fail(err, job->errorString().toUtf8());
    }
    return 0;
}

// Parses `url`; reports an error through `err` and returns false if it is not a valid URL.
static bool parse_url(const char *url, QUrl *out, char **err)
{
    *out = QUrl(QString::fromUtf8(url));
    if (out->isValid()) {
        return true;
    }
    fail(err, "invalid URL");
    return false;
}

extern "C" int markite_kio_read(const char *url, char **data, long long *len, long long max_len, char **err)
{
    *data = nullptr;
    *len = 0;
    *err = nullptr;
    QUrl target;
    if (!parse_url(url, &target, err)) {
        return 1;
    }
    TRACE("read: storedGet %s", shown(target).constData());
    auto *job = KIO::storedGet(target);
    // Stop the transfer as soon as the announced or received size passes the limit, instead of buffering it all.
    bool tooBig = false;
    auto check = [&](KJob *, qulonglong size) {
        if (!tooBig && size > static_cast<qulonglong>(max_len)) {
            tooBig = true;
            TRACE("read: %llu bytes exceeds the limit, killing the job", size);
            job->kill(KJob::EmitResult);
        }
    };
    QObject::connect(job, &KJob::totalSize, job, check);
    QObject::connect(job, &KJob::processedSize, job, check);
    if (int rc = finish(job, err); rc != 0) {
        if (tooBig) {
            std::free(*err);
            return fail(err, "file is larger than the size limit");
        }
        return rc;
    }
    const QByteArray bytes = job->data();
    TRACE("read: ok, %lld bytes", static_cast<long long>(bytes.size()));
    *data = dup_bytes(bytes);
    if (!*data) {
        return fail(err, "out of memory");
    }
    *len = static_cast<long long>(bytes.size());
    return 0;
}

extern "C" int markite_kio_write(const char *url, const char *data, long long len, char **err)
{
    *err = nullptr;
    if (len < 0 || (len > 0 && !data)) {
        return fail(err, "invalid buffer");
    }
    QUrl target;
    if (!parse_url(url, &target, err)) {
        return 1;
    }
    TRACE("write: storedPut %lld bytes to %s (Overwrite, no progress UI)", len, shown(target).constData());
    auto *job = KIO::storedPut(QByteArray(data, static_cast<qsizetype>(len)), target, -1,
                               KIO::Overwrite | KIO::HideProgressInfo);
    return finish(job, err);
}
