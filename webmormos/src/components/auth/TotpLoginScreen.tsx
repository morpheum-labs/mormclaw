import { useState } from 'react';
import { QRCodeSVG } from 'qrcode.react';
import { getTotpEnrollment } from '../../lib/api';
import { t } from '../../lib/i18n';

interface TotpLoginScreenProps {
  onLogin: (code: string) => Promise<void>;
  onUsePairingInstead: () => void;
}

export function TotpLoginScreen({
  onLogin,
  onUsePairingInstead,
}: TotpLoginScreenProps) {
  const [code, setCode] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [setupExpanded, setSetupExpanded] = useState(false);
  const [setupUri, setSetupUri] = useState<string | null>(null);
  const [setupLoading, setSetupLoading] = useState(false);
  const [setupError, setSetupError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');
    try {
      await onLogin(code);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Invalid code');
    } finally {
      setLoading(false);
    }
  };

  const handleSetupAuthenticator = async () => {
    if (setupUri) {
      setSetupExpanded(!setupExpanded);
      return;
    }
    setSetupLoading(true);
    setSetupError('');
    try {
      const data = await getTotpEnrollment();
      setSetupUri(data.otpauth_uri);
      setSetupExpanded(true);
    } catch (err: unknown) {
      setSetupError(err instanceof Error ? err.message : 'Setup failed');
    } finally {
      setSetupLoading(false);
    }
  };

  return (
    <div className="pairing-shell min-h-screen flex items-center justify-center px-4">
      <div className="pairing-card w-full max-w-md rounded-2xl p-8">
        <div className="text-center mb-6">
          <h1 className="mb-2 text-2xl font-semibold tracking-[0.16em]">MormOS</h1>
          <p className="text-sm text-[#9bb8e8]">{t('auth.totp_login_subtitle')}</p>
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
          {error && (
            <p className="mb-4 text-center text-sm text-rose-300">{error}</p>
          )}
          <button
            type="submit"
            disabled={loading || code.length < 6}
            className="electric-button w-full rounded-xl py-3 font-medium text-white disabled:opacity-50"
          >
            {loading ? '...' : t('auth.sign_in')}
          </button>
        </form>

        <div className="mt-6 flex flex-col items-center gap-2">
          <button
            type="button"
            onClick={onUsePairingInstead}
            className="text-xs text-[#9bb8e8]/80 hover:text-[#9bb8e8]"
          >
            {t('auth.use_pairing_instead')}
          </button>
          <button
            type="button"
            onClick={handleSetupAuthenticator}
            disabled={setupLoading}
            className="text-xs text-[#9bb8e8]/80 hover:text-[#9bb8e8] disabled:opacity-50"
          >
            {setupLoading ? '...' : t('auth.setup_authenticator')}
          </button>
        </div>

        {setupExpanded && setupUri && (
          <div className="mt-6 p-4 rounded-xl border border-[#29509c] bg-[#071228]/30">
            <p className="text-xs text-[#9bb8e8] mb-3">
              {t('auth.totp_enrollment_subtitle')}
            </p>
            <div className="flex justify-center">
              <QRCodeSVG value={setupUri} size={180} level="M" />
            </div>
            {setupError && (
              <p className="mt-3 text-center text-sm text-rose-300">{setupError}</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
