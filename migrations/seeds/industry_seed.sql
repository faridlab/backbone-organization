-- Industry reference data — the 21 top-level KBLI categories.
--
-- KBLI (Klasifikasi Baku Lapangan Usaha Indonesia) is the official Indonesian
-- business-activity classification, BPS Regulation 2/2020. Its top level is 21
-- categories lettered A through U, which is exactly what the `kbli_sector` enum
-- on this table already declares — the column was modelled for this list, and
-- only the data was missing.
--
-- Titles are the official Indonesian ones. Sub-categories (two-digit divisions
-- down to five-digit activities, 1,790 codes in all) hang off these through
-- parent_id and are not seeded here: a tenant adds the ones it actually
-- registers under.
--
-- Deterministic ids so a re-seed, or a second environment, resolves the same
-- row. org_unit_id is filled by the composing service's decorator from the
-- session's acting unit; this file names no tenancy.

INSERT INTO organization.industries (id, code, name, sector, parent_id, status, metadata) VALUES
  ('a0000000-0000-4b11-8000-00000000000a', 'A', 'Pertanian, Kehutanan dan Perikanan', 'a', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-00000000000b', 'B', 'Pertambangan dan Penggalian', 'b', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-00000000000c', 'C', 'Industri Pengolahan', 'c', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-00000000000d', 'D', 'Pengadaan Listrik, Gas, Uap/Air Panas dan Udara Dingin', 'd', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-00000000000e', 'E', 'Pengelolaan Air, Pengelolaan Air Limbah, Pengelolaan dan Daur Ulang Sampah, dan Aktivitas Remediasi', 'e', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-00000000000f', 'F', 'Konstruksi', 'f', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-000000000010', 'G', 'Perdagangan Besar dan Eceran; Reparasi dan Perawatan Mobil dan Sepeda Motor', 'g', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-000000000011', 'H', 'Pengangkutan dan Pergudangan', 'h', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-000000000012', 'I', 'Penyediaan Akomodasi dan Penyediaan Makan Minum', 'i', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-000000000013', 'J', 'Informasi dan Komunikasi', 'j', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-000000000014', 'K', 'Aktivitas Keuangan dan Asuransi', 'k', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-000000000015', 'L', 'Real Estat', 'l', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-000000000016', 'M', 'Aktivitas Profesional, Ilmiah dan Teknis', 'm', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-000000000017', 'N', 'Aktivitas Penyewaan dan Sewa Guna Usaha Tanpa Hak Opsi, Ketenagakerjaan, Agen Perjalanan dan Penunjang Usaha Lainnya', 'n', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-000000000018', 'O', 'Administrasi Pemerintahan, Pertahanan dan Jaminan Sosial Wajib', 'o', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-000000000019', 'P', 'Pendidikan', 'p', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-00000000001a', 'Q', 'Aktivitas Kesehatan Manusia dan Aktivitas Sosial', 'q', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-00000000001b', 'R', 'Kesenian, Hiburan dan Rekreasi', 'r', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-00000000001c', 'S', 'Aktivitas Jasa Lainnya', 's', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-00000000001d', 'T', 'Aktivitas Rumah Tangga sebagai Pemberi Kerja; Aktivitas yang Menghasilkan Barang dan Jasa oleh Rumah Tangga yang Digunakan untuk Memenuhi Kebutuhan Sendiri', 't', NULL, 'active', '{}'::jsonb),
  ('a0000000-0000-4b11-8000-00000000001e', 'U', 'Aktivitas Badan Internasional dan Badan Ekstra Internasional Lainnya', 'u', NULL, 'active', '{}'::jsonb)
ON CONFLICT (id) DO NOTHING;
