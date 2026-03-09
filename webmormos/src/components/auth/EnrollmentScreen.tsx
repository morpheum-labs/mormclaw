import { useState, useEffect } from 'react';
import { QRCodeSVG } from 'qrcode.react';
import { getTotpEnrollment } from '../../lib/api';
import { t } from '../../lib/i18n';

interface EnrollmentScreenProps {
  onLogin: (code: string) => Promise<void>;
  onUsePairingInstead: () => void;
}

export function EnrollmentScreen({
  onLogin,
  onUsePairingInstead,
}: EnrollmentScreenProps) {
  const [otpauthUri, setOtpauthUri] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [enrollError, setEnrollError] = useState('');
  const [code, setCode] = useState('');
  const [submitError, setSubmitError] = useState('');
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getTotpEnrollment()
      .then((data) => {
        if (!cancelled) {
          setOtpauthUri(data.otpauth_uri);
          setEnrollError('');
        }
      })
      .catch(() => {
        if (!cancelled) {
          setEnrollError(t('auth.enrollment_remote_error'));
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    setSubmitError('');
    try {
      await onLogin(code);
    } catch (err: unknown) {
      setSubmitError(err instanceof Error ? err.message : 'Invalid code');
    } finally {
      setSubmitting(false);
    }
  };

  if (loading) {
    return (
      <div className="pairing-shell min-h-screen flex items-center justify-center px-4">
        <div className="pairing-card w-full max-w-md rounded-2xl p-8">
          <div className="flex flex-col items-center gap-3">
            <div className="electric-loader h-10 w-10 rounded-full" />
            <p className="text-[#a7c4f3]">{t('auth.loading_qr')}</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="pairing-shell min-h-screen flex items-center justify-center px-4">
      <div className="pairing-card w-full max-w-md rounded-2xl p-8">
        <div className="text-center mb-6">
          <h1 className="mb-2 text-2xl font-semibold tracking-[0.16em]">MormOS</h1>
          <p className="text-sm text-[#9bb8e8]">
            {t('auth.totp_enrollment_subtitle')}
          </p>
        </div>

        {enrollError ? (
          <div className="mb-6 p-4 rounded-xl bg-rose-900/30 border border-rose-500/50">
            <p className="text-sm text-rose-300 mb-3">{enrollError}</p>
            <button
              type="button"
              onClick={onUsePairingInstead}
              className="text-xs text-[#9bb8e8]/80 hover:text-[#9bb8e8]"
            >
              {t('auth.use_pairing_instead')}
            </button>
          </div>
        ) : otpauthUri ? (
          <>
            <div className="flex justify-center mb-6">
              <div className="p-4 rounded-xl border border-[#29509c] bg-[#071228]/50">
                <QRCodeSVG value={otpauthUri} size={200} level="M" />
              </div>
            </div>
            <form onSubmit={handleSubmit}>
              <input
                type="text"
                value={code}
                onChange={(e) => setCode(e.target.value)}
                placeholder="6-digit code"
                className="w-full rounded-xl border border-[#29509c] bg-[#071228]/90 px-4 py-3 text-center text-2xl tracking-[0.35em] text-white focus:border-[#4f83ff] focus:outline-none mb-4"
                maxLength={6}
                autoFocus
              />
              {submitError && (
                <p className="mb-4 text-center text-sm text-rose-300">{submitError}</p>
              )}
              <button
                type="submit"
                disabled={submitting || code.length < 6}
                className="electric-button w-full rounded-xl py-3 font-medium text-white disabled:opacity-50"
              >
                {submitting ? '...' : t('auth.confirm')}
              </button>
            </form>
          </>
        ) : null}

        <p className="mt-6 text-center">
          <button
            type="button"
            onClick={onUsePairingInstead}
            className="text-xs text-[#9bb8e8]/80 hover:text-[#9bb8e8]"
          >
            {t('auth.use_pairing_instead')}
          </button>
        </p>
      </div>
    </div>
  );
}
