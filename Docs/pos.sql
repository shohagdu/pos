-- phpMyAdmin SQL Dump
-- version 5.1.1deb5ubuntu1
-- https://www.phpmyadmin.net/
--
-- Host: localhost
-- Generation Time: Oct 26, 2026 at 02:22 AM
-- Server version: 8.0.46-0ubuntu0.22.04.4
-- PHP Version: 8.2.34

SET SQL_MODE = "NO_AUTO_VALUE_ON_ZERO";
START TRANSACTION;
SET time_zone = "+00:00";


/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
/*!40101 SET @OLD_CHARACTER_SET_RESULTS=@@CHARACTER_SET_RESULTS */;
/*!40101 SET @OLD_COLLATION_CONNECTION=@@COLLATION_CONNECTION */;
/*!40101 SET NAMES utf8mb4 */;

--
-- Database: `pos`
--

-- --------------------------------------------------------

--
-- Table structure for table `account_transactions`
--

CREATE TABLE `account_transactions` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `account_id` binary(16) NOT NULL,
  `branch_id` binary(16) DEFAULT NULL,
  `txn_type` varchar(20) NOT NULL,
  `amount` decimal(18,2) NOT NULL,
  `balance_after` decimal(18,2) NOT NULL,
  `business_date` date NOT NULL,
  `reference_type` varchar(30) NOT NULL,
  `reference_id` binary(16) NOT NULL,
  `reference_no` varchar(40) DEFAULT NULL,
  `note` varchar(500) DEFAULT NULL,
  `created_by` binary(16) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `account_transactions`
--

INSERT INTO `account_transactions` (`tenant_id`, `id`, `account_id`, `branch_id`, `txn_type`, `amount`, `balance_after`, `business_date`, `reference_type`, `reference_id`, `reference_no`, `note`, `created_by`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9ff296756b8526d73ece34b946, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_in', '2.00', '2.00', '2026-10-18', 'payment', 0x01a14d9ff292766ea402b9f17b997f05, 'RCV-2610-00001', 'Sale INV-2610-00001', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-18 06:08:10.390947'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14da336d576758db341868cfa1ad6, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_in', '6.00', '8.00', '2026-10-18', 'payment', 0x01a14da336d171c196e49427b8d3097a, 'RCV-2610-00002', 'Sale INV-2610-00002', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-18 06:11:44.469530'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d33c7f748ea4d2351541ef5cfa, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_in', '4.00', '12.00', '2026-10-19', 'payment', 0x01a151d33c7c7573ba30980ef714912f, 'RCV-2610-00003', 'Sale INV-2610-00003', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:42:40.512544'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d724e77170af17cee4a61c48f3, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_out', '-100.00', '-88.00', '2026-10-19', 'payment', 0x01a151d724e47199b13de8df2c8f054e, 'PMT-2610-00001', 'Payment to Mr Kashem', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:46:56.615726'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d7f75076aaa40d28c32aa5d3cc, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'expense', '-1.00', '-89.00', '2026-10-19', 'payment', 0x01a151d7f74d7268837546ff0cb78b63, 'PMT-2610-00002', 'Marketing', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:47:50.481345'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1521a06aa72d185162a3338d30217, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_out', '-2.10', '-91.10', '2026-10-19', 'payment', 0x01a1521a06a5741281ff7a324443dec9, 'PMT-2610-00003', 'Refund RET-2610-00001 for INV-2610-00003', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 02:59:59.787978'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15563751d764b910aef1eb4b5e247, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'transfer_out', '-10.00', '-101.10', '2026-10-20', 'account_transfer', 0x01a155637515710e90247d06ca9ef0ad, 'TRF-2610-00001', 'To Mobile Wallet', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 18:19:03.842813'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15563766976429b0cf30e1add5530, 0x11f1cab9ed515b95ab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'transfer_in', '10.00', '10.00', '2026-10-20', 'account_transfer', 0x01a155637515710e90247d06ca9ef0ad, 'TRF-2610-00001', 'From Cash', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 18:19:04.169699'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9fd9f37128b524c651798f8ba7, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_in', '4.00', '-97.10', '2026-10-21', 'payment', 0x01a15a9fd9ef7012946c3bc56619a222, 'RCV-2610-00004', 'Sale INV-2610-00004', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-20 18:43:07.892454'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734deeab7332aaaf3b1c4efaa44d, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'income', '11.00', '-86.10', '2026-10-25', 'payment', 0x01a1734dee1c75d39453d6538f1ea7ed, 'RCV-2610-00005', 'Rent Received — 1', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 13:44:09.647139'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734e406d70f088b6f0e90c5f00ba, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'transfer_out', '-1.00', '-87.10', '2026-10-25', 'account_transfer', 0x01a1734e406b71b8875364222b7b6a82, 'TRF-2610-00002', 'To Bank Account', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 13:44:30.574658'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734e406f7468b6eba0ac2777cf37, 0x11f1cab9ed5155c9ab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'transfer_in', '1.00', '1.00', '2026-10-25', 'account_transfer', 0x01a1734e406b71b8875364222b7b6a82, 'TRF-2610-00002', 'From Cash', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 13:44:30.575965'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c3c90970de86ad5edc43b7e338, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_in', '2.00', '-85.10', '2026-10-25', 'payment', 0x01a173c3c83b77c6987217e24cc6448b, 'RCV-2610-00006', 'Sale INV-2610-00005', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 15:52:53.261585'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cb7cb5751ab6df40ff499177be, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_in', '2.00', '-83.10', '2026-10-25', 'payment', 0x01a173cb7cb270779f68efc75313eb1b, 'RCV-2610-00007', 'Sale INV-2610-00006', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:01:18.006446'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cbf17975b9b82c29a070422038, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_in', '2.00', '-81.10', '2026-10-25', 'payment', 0x01a173cbf17573d18ec25959c5d447e8, 'RCV-2610-00008', 'Sale INV-2610-00007', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:01:47.897791'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d0cf1e72a9886ce77d7e61344e, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_in', '2.00', '-79.10', '2026-10-25', 'payment', 0x01a173d0cf1b75a0a5d16d55080ed558, 'RCV-2610-00009', 'Sale INV-2610-00008', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:07:06.782498'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e2d3c971db88e04fa77362974a, 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'payment_out', '-4.00', '-83.10', '2026-10-25', 'payment', 0x01a173e2d3c375b891737cae376653cb, 'PMT-2610-00004', 'Payment to Mr Kashem', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:26:47.626625');

--
-- Triggers `account_transactions`
--
DELIMITER $$
CREATE TRIGGER `trg_account_transactions_ao_del` BEFORE DELETE ON `account_transactions` FOR EACH ROW BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_account_transactions_ao_upd` BEFORE UPDATE ON `account_transactions` FOR EACH ROW BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_account_transactions_tg_ins` BEFORE INSERT ON `account_transactions` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `account_transfers`
--

CREATE TABLE `account_transfers` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `transfer_no` varchar(40) NOT NULL,
  `from_account_id` binary(16) NOT NULL,
  `to_account_id` binary(16) NOT NULL,
  `amount` decimal(18,2) NOT NULL,
  `fee_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `business_date` date NOT NULL,
  `reference` varchar(100) DEFAULT NULL,
  `note` varchar(500) DEFAULT NULL,
  `status` varchar(10) NOT NULL DEFAULT 'posted',
  `created_by` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `account_transfers`
--

INSERT INTO `account_transfers` (`tenant_id`, `id`, `branch_id`, `transfer_no`, `from_account_id`, `to_account_id`, `amount`, `fee_amount`, `business_date`, `reference`, `note`, `status`, `created_by`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a155637515710e90247d06ca9ef0ad, 0x01a14c8ff30876da9be77f7076393059, 'TRF-2610-00001', 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed515b95ab00bc542fed17aa, '10.00', '0.00', '2026-10-20', 'test', 't', 'posted', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 18:19:03.831543'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734e406b71b8875364222b7b6a82, 0x01a14c8ff30876da9be77f7076393059, 'TRF-2610-00002', 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5155c9ab00bc542fed17aa, '1.00', '0.00', '2026-10-25', NULL, NULL, 'posted', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 13:44:30.571842');

--
-- Triggers `account_transfers`
--
DELIMITER $$
CREATE TRIGGER `trg_account_transfers_tg_del` BEFORE DELETE ON `account_transfers` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_account_transfers_tg_ins` BEFORE INSERT ON `account_transfers` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_account_transfers_tg_upd` BEFORE UPDATE ON `account_transfers` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `audit_logs`
--

CREATE TABLE `audit_logs` (
  `id` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `tenant_id` binary(16) DEFAULT NULL,
  `actor_type` varchar(20) NOT NULL,
  `actor_id` binary(16) DEFAULT NULL,
  `action` varchar(60) NOT NULL,
  `entity_type` varchar(40) DEFAULT NULL,
  `entity_id` binary(16) DEFAULT NULL,
  `branch_id` binary(16) DEFAULT NULL,
  `before_data` json DEFAULT NULL,
  `after_data` json DEFAULT NULL,
  `reason` varchar(500) DEFAULT NULL,
  `ip_address` varbinary(16) DEFAULT NULL,
  `user_agent` varchar(500) DEFAULT NULL,
  `request_id` varchar(64) DEFAULT NULL
) ;

--
-- Dumping data for table `audit_logs`
--

INSERT INTO `audit_logs` (`id`, `created_at`, `tenant_id`, `actor_type`, `actor_id`, `action`, `entity_type`, `entity_id`, `branch_id`, `before_data`, `after_data`, `reason`, `ip_address`, `user_agent`, `request_id`) VALUES
(0x01a14c8ff3d076378c10e72d9f2d0751, '2026-10-18 01:11:04.915072', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'tenant.provisioned', 'tenant', 0x01a14c8ff2df764db434b8b65d8bac97, NULL, NULL, '{\"name\": \"Test Shop\", \"slug\": \"test-shop\", \"business_type\": \"grocery\"}', NULL, 0x7f000001, 'curl/7.81.0', '01a14c8f-f216-75f3-ad27-2f9f7e232987'),
(0x01a14c91a60172ffaee33e98ea35178f, '2026-10-18 01:12:56.066019', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'product.created', 'product', 0x01a14c91a5fa72269dd24dd4ee7deede, NULL, NULL, '{\"id\": \"01a14c91-a5fa-7226-9dd2-4dd4ee7deede\", \"sku\": \"Ata-001\", \"name\": \"Ata 800\", \"status\": \"active\", \"name_bn\": \"অাটা\", \"unit_id\": \"01a14c8f-f2f9-761e-b9f6-38875ad3ce54\", \"version\": 1, \"barcodes\": [], \"brand_id\": null, \"brand_name\": null, \"created_at\": \"2026-10-18T01:12:56.059144Z\", \"updated_at\": \"2026-10-18T01:12:56.059144Z\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"category_id\": null, \"description\": null, \"tax_rate_id\": \"01a14c8f-f2ed-71d6-a7c4-621fce01d7eb\", \"product_type\": \"service\", \"tax_rate_pct\": \"5.0000\", \"category_name\": null, \"reorder_level\": null, \"selling_price\": \"2.0000\", \"allow_fraction\": true, \"purchase_price\": \"2.0000\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"wholesale_price\": \"2.0000\", \"min_selling_price\": \"2.0000\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '01a14c91-a5e4-73cf-a447-138a322238e5'),
(0x01a14d9ff29d75f1819f1bbb8c10fb6a, '2026-10-18 06:08:10.397716', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'sale.completed', 'sale', 0x01a14d9ff28c702f800284db35995a81, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"lines\": 1, \"customer\": \"Walk-in Customer\", \"due_total\": \"0.00\", \"invoice_no\": \"INV-2610-00001\", \"paid_total\": \"2\", \"grand_total\": \"2.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '01a14d9f-f25e-73a1-a74d-5968aea90607'),
(0x01a14da336da76a9932fe2c519237af9, '2026-10-18 06:11:44.474954', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'sale.completed', 'sale', 0x01a14da336ca723dac4fec5f7d9b1aeb, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"lines\": 1, \"customer\": \"Walk-in Customer\", \"due_total\": \"0.00\", \"invoice_no\": \"INV-2610-00002\", \"paid_total\": \"6\", \"grand_total\": \"6.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '01a14da3-36bf-757b-814e-7be015534e23'),
(0x01a151d33c8874c9ac57f70a88f9810a, '2026-10-19 01:42:40.520658', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'sale.completed', 'sale', 0x01a151d33c73715180219c1cedd1bb46, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"lines\": 1, \"customer\": \"Walk-in Customer\", \"due_total\": \"0.00\", \"invoice_no\": \"INV-2610-00003\", \"paid_total\": \"4\", \"grand_total\": \"4.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '01a151d3-3c61-71d7-8977-f15a3de16622'),
(0x01a151d68d15739593517891d55912f5, '2026-10-19 01:46:17.751741', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'supplier.created', 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, NULL, NULL, '{\"id\": \"01a151d6-8d09-74f8-bbb2-203ca6b1e0e0\", \"code\": \"S000001\", \"name\": \"Mr Kashem\", \"email\": \"omarshohag93@gmail.com\", \"notes\": \"Hello the world\", \"phone\": null, \"status\": \"active\", \"address\": \"Lemua, Feni\", \"version\": 1, \"created_at\": \"2026-10-19T01:46:17.738407Z\", \"updated_at\": \"2026-10-19T01:46:17.740274Z\", \"contact_person\": \"01839707645\", \"current_balance\": \"100.00\", \"opening_balance\": \"100.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '01a151d6-8d00-755e-ab5d-32ced6d6b3ea'),
(0x01a151d724ee723a9e87ea456ed272de, '2026-10-19 01:46:56.622924', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'supplier.payment_made', 'payment', 0x01a151d724e47199b13de8df2c8f054e, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"amount\": \"100\", \"supplier\": \"Mr Kashem\", \"payment_no\": \"PMT-2610-00001\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '01a151d7-24dc-719f-83b2-9a9d9b497f0f'),
(0x01a151d7f75972d6b3e59e747e7872e2, '2026-10-19 01:47:50.489777', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'expense.created', 'expense', 0x01a151d7f749779b8799b76ffb0c0d82, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"amount\": \"1\", \"account\": \"Cash\", \"category\": \"Marketing\", \"expense_no\": \"EXP-2610-00001\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '01a151d7-f735-7560-bc2a-6f89ece9069b'),
(0x01a151ea3646761fa0a6787f40d20cdc, '2026-10-19 02:07:46.247687', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'tenant.data_exported', NULL, NULL, NULL, NULL, '{\"tables\": 41}', NULL, 0x7f000001, 'curl/7.81.0', '01a151ea-355f-7157-af89-4c16e07201d6'),
(0x01a1521a06b475c09ed0f60304ecbde2, '2026-10-19 02:59:59.796953', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'sale.returned', 'sale_return', 0x01a1521a069874d2a5190fdfcd9f0412, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"total\": \"2.1\", \"credited\": \"0\", \"refunded\": \"2.1\", \"return_no\": \"RET-2610-00001\", \"invoice_no\": \"INV-2610-00003\", \"applied_to_due\": \"0.00\"}', 'test', 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '01a1521a-0685-72b6-bd37-b39d423e88d9'),
(0x01a155637718765a8b87f3b38a95eceb, '2026-10-19 18:19:04.349027', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'account_transfer.created', 'account_transfer', 0x01a155637515710e90247d06ca9ef0ad, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"to\": \"Mobile Wallet\", \"fee\": \"0\", \"from\": \"Cash\", \"amount\": \"10\", \"transfer_no\": \"TRF-2610-00001\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '01a15563-74c3-71d9-8c9a-10dbaa69f91c'),
(0x01a15a9ee77d7112984d5848adf696e3, '2026-10-20 18:42:05.824572', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'onboarding.step_completed', NULL, NULL, NULL, NULL, '{\"step\": \"regional\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a15a9e-e776-7497-b912-9764fb6abcff'),
(0x01a15a9ef4a1756ca82c9572de431f6f, '2026-10-20 18:42:09.186581', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'onboarding.step_completed', NULL, NULL, NULL, NULL, '{\"step\": \"branch\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a15a9e-f49b-71ea-9fb4-dad45adcc9e1'),
(0x01a15a9efced7038ac4c130a342a65fd, '2026-10-20 18:42:11.309429', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'onboarding.step_completed', NULL, NULL, NULL, NULL, '{\"step\": \"invoice\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a15a9e-fce9-72de-897d-2de6a36883ff'),
(0x01a15a9f23ef750184ed57160bfa43bd, '2026-10-20 18:42:21.295519', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'onboarding.step_completed', NULL, NULL, NULL, NULL, '{\"step\": \"products\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a15a9f-23d8-7787-9f31-9499ce96a6dc'),
(0x01a15a9f2f0a74a4bbdd7aa3c69fbdaf, '2026-10-20 18:42:24.140552', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'onboarding.step_completed', NULL, NULL, NULL, NULL, '{\"step\": \"staff\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a15a9f-2ef0-72fd-a719-c888c22bb2d3'),
(0x01a15a9fda5b773698c4cec205c7d134, '2026-10-20 18:43:07.996228', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'sale.completed', 'sale', 0x01a15a9fd9e6747db53070ca2ec2e354, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"lines\": 1, \"customer\": \"Walk-in Customer\", \"due_total\": \"0.00\", \"invoice_no\": \"INV-2610-00004\", \"paid_total\": \"4\", \"grand_total\": \"4.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a15a9f-d9d5-741a-aa2f-a785a0a763e9'),
(0x01a1734def3777dc9e6e84d657639e55, '2026-10-25 13:44:09.786240', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'income.created', 'other_income', 0x01a1734dee1076a799b9747b07b9c05e, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"amount\": \"11\", \"account\": \"Cash\", \"category\": \"Rent Received\", \"income_no\": \"INC-2610-00001\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a1734d-eca3-70ab-964e-ef5a3892b88b'),
(0x01a1734e4076714abbe64cc25df3952d, '2026-10-25 13:44:30.583213', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'account_transfer.created', 'account_transfer', 0x01a1734e406b71b8875364222b7b6a82, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"to\": \"Bank Account\", \"fee\": \"0\", \"from\": \"Cash\", \"amount\": \"1\", \"transfer_no\": \"TRF-2610-00002\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a1734e-3fd6-71b9-859d-990097109052'),
(0x01a173704f457164b11cbb36a59c6f49, '2026-10-25 14:21:42.600191', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'subscription.invoice_created', 'subscription_invoice', 0x01a173704e22774cbd647e556fe18b0c, NULL, NULL, '{\"plan\": \"business\", \"cycle\": \"annual\", \"total\": \"25398.00\", \"invoice_no\": \"SUB-2026-000001\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a17370-4bbd-744c-90b6-2b5cceb9f4ef'),
(0x01a17370906773bdbd22084af12ca3d7, '2026-10-25 14:21:59.273249', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'subscription.payment_claimed', 'subscription_payment_claim', 0x01a173708fde70619e13105784332a21, NULL, NULL, '{\"amount\": \"25398\", \"method\": \"bkash\", \"transaction_ref\": \"2255\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a17370-8fa6-76f2-8d0c-a11c749291f7'),
(0x01a17386777072028b9c8a55663a71a7, '2026-10-25 14:45:54.673625', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'register.created', 'cash_register', 0x01a17386776f7022a112740b2ae9f46a, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"name\": \"Hello Test Register\", \"cash_account_id\": \"11f1cab9-ed51-475a-ab00-bc542fed17aa\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a17386-7766-77f8-a613-c6b210e96cee'),
(0x01a17386a31d7461b4f3371804163d8b, '2026-10-25 14:46:05.853920', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'shift.opened', 'register_shift', 0x01a17386a31c75dfb6c82efa53e9ff79, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"register_id\": \"01a17386-776f-7022-a112-740b2ae9f46a\", \"opening_cash\": \"11\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a17386-a317-7073-a257-a8f5c6141bdb'),
(0x01a17389c82476a18c3dcfada5b4fae2, '2026-10-25 14:49:31.941235', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'customer.created', 'customer', 0x01a17389c81c76ec9bb583d2246c37ba, NULL, NULL, '{\"id\": \"01a17389-c81c-76ec-9bb5-83d2246c37ba\", \"code\": \"C000001\", \"name\": \"Jashi\", \"email\": null, \"notes\": null, \"phone\": null, \"status\": \"active\", \"address\": null, \"version\": 1, \"created_at\": \"2026-10-25T14:49:31.934625Z\", \"is_walk_in\": false, \"updated_at\": \"2026-10-25T14:49:31.934625Z\", \"credit_limit\": null, \"current_balance\": \"0.00\", \"opening_balance\": \"0.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a17389-c803-7645-ad03-b05eca27b032'),
(0x01a1738a084b72fea3f55c75ddc7a71d, '2026-10-25 14:49:48.364197', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'customer.created', 'customer', 0x01a1738a084873a59ddeb03f71ea2ae5, NULL, NULL, '{\"id\": \"01a1738a-0848-73a5-9dde-b03f71ea2ae5\", \"code\": \"C000002\", \"name\": \"Jashim\", \"email\": null, \"notes\": null, \"phone\": \"01839707645\", \"status\": \"active\", \"address\": null, \"version\": 1, \"created_at\": \"2026-10-25T14:49:48.361303Z\", \"is_walk_in\": false, \"updated_at\": \"2026-10-25T14:49:48.361303Z\", \"credit_limit\": null, \"current_balance\": \"0.00\", \"opening_balance\": \"0.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a1738a-0833-741a-8097-599807259703'),
(0x01a173c3cb9b71388723da8c000d5d71, '2026-10-25 15:52:53.918203', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'sale.completed', 'sale', 0x01a173c3c7a476a48a17e9cd82ddd428, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"lines\": 1, \"customer\": \"Walk-in Customer\", \"due_total\": \"0.00\", \"invoice_no\": \"INV-2610-00005\", \"paid_total\": \"2.00\", \"grand_total\": \"2.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173c3-c72b-72f4-9f13-27591f22b741'),
(0x01a173cb7cbc7410af589e15fdd140e0, '2026-10-25 16:01:18.012931', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'sale.completed', 'sale', 0x01a173cb7caa7227ac4c13cf122599af, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"lines\": 1, \"customer\": \"Walk-in Customer\", \"due_total\": \"0.00\", \"invoice_no\": \"INV-2610-00006\", \"paid_total\": \"2.00\", \"grand_total\": \"2.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173cb-7ca0-73b0-95d7-bd00eafb196b'),
(0x01a173cbf1807662b0b3ac9828193c2f, '2026-10-25 16:01:47.904979', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'sale.completed', 'sale', 0x01a173cbf16c77768110a202cf51cead, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"lines\": 1, \"customer\": \"Walk-in Customer\", \"due_total\": \"0.00\", \"invoice_no\": \"INV-2610-00007\", \"paid_total\": \"2.00\", \"grand_total\": \"2.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173cb-f15d-70f2-9bc6-7b67485345af'),
(0x01a173d0cf23714c9913ab36009c1155, '2026-10-25 16:07:06.787402', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'sale.completed', 'sale', 0x01a173d0cf157236a45375e8a5272e85, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"lines\": 1, \"customer\": \"Walk-in Customer\", \"due_total\": \"0.00\", \"invoice_no\": \"INV-2610-00008\", \"paid_total\": \"2\", \"grand_total\": \"2.00\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173d0-cf05-762b-a95f-302ea2004402'),
(0x01a173d3e50f72848c6bc45654ddbc02, '2026-10-25 16:10:29.007247', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'subscription.invoice_created', 'subscription_invoice', 0x01a173d3e50d74639e2eaa3b65f40694, NULL, NULL, '{\"plan\": \"business\", \"cycle\": \"monthly\", \"total\": \"2490.00\", \"invoice_no\": \"SUB-2026-000002\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173d3-e4f2-7652-ba0e-e25e5871e83e'),
(0x01a173d400b87345bb61768586900735, '2026-10-25 16:10:36.088955', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'subscription.payment_claimed', 'subscription_payment_claim', 0x01a173d400b7775ab13620dc13f8c5f7, NULL, NULL, '{\"amount\": \"2490\", \"method\": \"bkash\", \"transaction_ref\": \"545454\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173d4-00ae-7193-90e8-b34296aeb713'),
(0x01a173da39a37742a8d8b0d3145b9316, '2026-10-25 16:17:23.877072', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'category.created', 'category', 0x01a173da393271caa334171028125df8, NULL, NULL, '{\"id\": \"01a173da-3932-71ca-a334-171028125df8\", \"name\": \"Grocery\", \"depth\": 0, \"status\": \"active\", \"parent_id\": null, \"sort_order\": 0, \"product_count\": 0}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173da-391f-766f-83ff-de555ec802ab'),
(0x01a173da52dc702d8d13f510f1e3d3b3, '2026-10-25 16:17:30.334712', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'brand.created', 'brand', 0x01a173da52d976e4837a27669b51c389, NULL, NULL, '{\"id\": \"01a173da-52d9-76e4-837a-27669b51c389\", \"name\": \"Fresh\", \"status\": \"active\", \"product_count\": 0}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173da-52ca-72cb-b31c-4568a6afaf7c'),
(0x01a173dac36f714496775cd8bc03138f, '2026-10-25 16:17:59.152264', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'product.updated', 'product', 0x01a14c91a5fa72269dd24dd4ee7deede, NULL, '{\"id\": \"01a14c91-a5fa-7226-9dd2-4dd4ee7deede\", \"sku\": \"Ata-001\", \"name\": \"Ata 800\", \"status\": \"active\", \"name_bn\": \"অাটা\", \"unit_id\": \"01a14c8f-f2f9-761e-b9f6-38875ad3ce54\", \"version\": 1, \"barcodes\": [], \"brand_id\": null, \"brand_name\": null, \"created_at\": \"2026-10-18T01:12:56.059144Z\", \"updated_at\": \"2026-10-18T01:12:56.059144Z\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"category_id\": null, \"description\": null, \"tax_rate_id\": \"01a14c8f-f2ed-71d6-a7c4-621fce01d7eb\", \"product_type\": \"service\", \"tax_rate_pct\": \"5.0000\", \"category_name\": null, \"image_file_id\": null, \"reorder_level\": null, \"selling_price\": \"2.0000\", \"allow_fraction\": true, \"purchase_price\": \"2.0000\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"wholesale_price\": \"2.0000\", \"min_selling_price\": \"2.0000\"}', '{\"id\": \"01a14c91-a5fa-7226-9dd2-4dd4ee7deede\", \"sku\": \"Ata-001\", \"name\": \"Ata 800\", \"status\": \"active\", \"name_bn\": \"অাটা\", \"unit_id\": \"01a14c8f-f2f9-761e-b9f6-38875ad3ce54\", \"version\": 2, \"barcodes\": [], \"brand_id\": null, \"brand_name\": null, \"created_at\": \"2026-10-18T01:12:56.059144Z\", \"updated_at\": \"2026-10-25T16:17:58.866133Z\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"category_id\": null, \"description\": null, \"tax_rate_id\": \"01a14c8f-f2ed-71d6-a7c4-621fce01d7eb\", \"product_type\": \"standard\", \"tax_rate_pct\": \"5.0000\", \"category_name\": null, \"image_file_id\": null, \"reorder_level\": null, \"selling_price\": \"2.0000\", \"allow_fraction\": true, \"purchase_price\": \"2.0000\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"wholesale_price\": \"2.0000\", \"min_selling_price\": \"2.0000\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173da-c239-7210-beef-38950ff2eb8d'),
(0x01a173daec57712db74fe9779f6ffa92, '2026-10-25 16:18:09.624149', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'product.updated', 'product', 0x01a14c91a5fa72269dd24dd4ee7deede, NULL, '{\"id\": \"01a14c91-a5fa-7226-9dd2-4dd4ee7deede\", \"sku\": \"Ata-001\", \"name\": \"Ata 800\", \"status\": \"active\", \"name_bn\": \"অাটা\", \"unit_id\": \"01a14c8f-f2f9-761e-b9f6-38875ad3ce54\", \"version\": 2, \"barcodes\": [], \"brand_id\": null, \"brand_name\": null, \"created_at\": \"2026-10-18T01:12:56.059144Z\", \"updated_at\": \"2026-10-25T16:17:58.866133Z\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"category_id\": null, \"description\": null, \"tax_rate_id\": \"01a14c8f-f2ed-71d6-a7c4-621fce01d7eb\", \"product_type\": \"standard\", \"tax_rate_pct\": \"5.0000\", \"category_name\": null, \"image_file_id\": null, \"reorder_level\": null, \"selling_price\": \"2.0000\", \"allow_fraction\": true, \"purchase_price\": \"2.0000\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"wholesale_price\": \"2.0000\", \"min_selling_price\": \"2.0000\"}', '{\"id\": \"01a14c91-a5fa-7226-9dd2-4dd4ee7deede\", \"sku\": \"Ata-001\", \"name\": \"Ata 800\", \"status\": \"active\", \"name_bn\": \"অাটা\", \"unit_id\": \"01a14c8f-f2f9-761e-b9f6-38875ad3ce54\", \"version\": 3, \"barcodes\": [], \"brand_id\": \"01a173da-52d9-76e4-837a-27669b51c389\", \"brand_name\": \"Fresh\", \"created_at\": \"2026-10-18T01:12:56.059144Z\", \"updated_at\": \"2026-10-25T16:18:09.619048Z\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"category_id\": \"01a173da-3932-71ca-a334-171028125df8\", \"description\": null, \"tax_rate_id\": \"01a14c8f-f2ed-71d6-a7c4-621fce01d7eb\", \"product_type\": \"standard\", \"tax_rate_pct\": \"5.0000\", \"category_name\": \"Grocery\", \"image_file_id\": null, \"reorder_level\": null, \"selling_price\": \"2.0000\", \"allow_fraction\": true, \"purchase_price\": \"2.0000\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"wholesale_price\": \"2.0000\", \"min_selling_price\": \"2.0000\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173da-ec3a-73cf-b5bc-69eed022c28e'),
(0x01a173e1bcb977099c048438ecb7aa26, '2026-10-25 16:25:36.188469', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'product.created', 'product', 0x01a173e1bc2f72d38cbf3f345a7e5305, NULL, NULL, '{\"id\": \"01a173e1-bc2f-72d3-8cbf-3f345a7e5305\", \"sku\": \"008\", \"name\": \"Pen\", \"status\": \"active\", \"name_bn\": \"কলম\", \"unit_id\": \"01a14c8f-f2f6-77f6-93a2-23fc85d990ab\", \"version\": 1, \"barcodes\": [\"008\"], \"brand_id\": \"01a173da-52d9-76e4-837a-27669b51c389\", \"brand_name\": \"Fresh\", \"created_at\": \"2026-10-25T16:25:36.049331Z\", \"updated_at\": \"2026-10-25T16:25:36.049331Z\", \"variant_id\": \"01a173e1-bc2f-72d3-8cbf-3f359dbef54a\", \"category_id\": \"01a173da-3932-71ca-a334-171028125df8\", \"description\": null, \"tax_rate_id\": \"01a14c8f-f2e9-708e-988f-f9bfb894aaf2\", \"product_type\": \"standard\", \"tax_rate_pct\": \"0.0000\", \"category_name\": \"Grocery\", \"image_file_id\": null, \"reorder_level\": \"100.000\", \"selling_price\": \"5.0000\", \"allow_fraction\": false, \"purchase_price\": \"4.0000\", \"track_inventory\": true, \"unit_short_name\": \"pcs\", \"wholesale_price\": \"4.5000\", \"min_selling_price\": \"60.0000\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173e1-bbf9-7644-a634-ad311a6bd051'),
(0x01a173e299b572f886fd52f9d2520830, '2026-10-25 16:26:32.759485', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'purchase.draft_saved', 'purchase', 0x01a173e29933711bbff0746df7a0951b, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"lines\": 1, \"grand_total\": \"4\", \"supplier_id\": \"01a151d6-8d09-74f8-bbb2-203ca6b1e0e0\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173e2-990a-756b-972b-0e5bce043c86'),
(0x01a173e29e3671eebe5dd7dffa1091e3, '2026-10-25 16:26:33.912792', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'purchase.posted', 'purchase', 0x01a173e29933711bbff0746df7a0951b, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"supplier\": \"Mr Kashem\", \"receipt_no\": \"GRN-2610-00001\", \"grand_total\": \"4.00\", \"purchase_no\": \"PUR-2610-00001\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173e2-9bea-75a5-beab-b331f939c184'),
(0x01a173e2d3d87549bcba2e8c6ca9008c, '2026-10-25 16:26:47.641817', 0x01a14c8ff2df764db434b8b65d8bac97, 'user', 0x01a14c8ff09475ea92a85fc5af151c9e, 'supplier.payment_made', 'payment', 0x01a173e2d3c375b891737cae376653cb, 0x01a14c8ff30876da9be77f7076393059, NULL, '{\"amount\": \"4\", \"supplier\": \"Mr Kashem\", \"payment_no\": \"PMT-2610-00004\"}', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '01a173e2-d3b0-73da-b6a2-bf23e01d498e');

--
-- Triggers `audit_logs`
--
DELIMITER $$
CREATE TRIGGER `trg_audit_logs_ao_del` BEFORE DELETE ON `audit_logs` FOR EACH ROW BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_audit_logs_ao_upd` BEFORE UPDATE ON `audit_logs` FOR EACH ROW BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `auth_events`
--

CREATE TABLE `auth_events` (
  `id` binary(16) NOT NULL,
  `user_id` binary(16) DEFAULT NULL,
  `tenant_id` binary(16) DEFAULT NULL,
  `event` varchar(40) NOT NULL,
  `identifier` varchar(254) DEFAULT NULL,
  `ip_address` varbinary(16) DEFAULT NULL,
  `user_agent` varchar(500) DEFAULT NULL,
  `metadata` json NOT NULL DEFAULT (json_object()),
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Dumping data for table `auth_events`
--

INSERT INTO `auth_events` (`id`, `user_id`, `tenant_id`, `event`, `identifier`, `ip_address`, `user_agent`, `metadata`, `created_at`) VALUES
(0x01a14c8ff0997303abd6ddbe3f9dd287, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'user.registered', NULL, 0x7f000001, 'curl/7.81.0', '{}', '2026-10-18 01:11:04.090270'),
(0x01a14c8ff16972ada81184116389c358, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'phone.verified', NULL, 0x7f000001, 'curl/7.81.0', '{}', '2026-10-18 01:11:04.299361'),
(0x01a14c8ff4a4777181b6bca938dd9b67, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 'tenant.switched', NULL, 0x7f000001, 'curl/7.81.0', '{}', '2026-10-18 01:11:05.124522'),
(0x01a14c900b577290b0bf8aef57540b12, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'login.success', NULL, 0x7f000001, 'curl/7.81.0', '{}', '2026-10-18 01:11:10.936008'),
(0x01a14c908f9676bc94765b8d0d6f60c0, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'login.success', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '{}', '2026-10-18 01:11:44.790174'),
(0x01a14cf8685b766bbe1e47fe14565e19, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 'refresh.reuse_detected', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '{\"revoked_sessions\": 1}', '2026-10-18 03:05:10.661586'),
(0x01a14d4ab2a0767891be28cd1cfe5e08, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'login.success', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '{}', '2026-10-18 04:35:03.456934'),
(0x01a14d9e6123750985878bfc2cbf33f2, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'login.success', NULL, 0x7f000001, 'curl/7.81.0', '{}', '2026-10-18 06:06:27.619792'),
(0x01a151e308f17226af72d2fb66b19761, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 'refresh.reuse_detected', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '{\"revoked_sessions\": 1}', '2026-10-19 01:59:56.065463'),
(0x01a151ea33707540a2a19b00be4e8714, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'login.success', NULL, 0x7f000001, 'curl/7.81.0', '{}', '2026-10-19 02:07:45.520476'),
(0x01a15204f9f572958e0cdc7c3c30ef07, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'login.success', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '{}', '2026-10-19 02:37:00.278098'),
(0x01a1570562f5737f9731dbf07d30d28c, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 'refresh.reuse_detected', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '{\"revoked_sessions\": 1}', '2026-10-20 01:55:33.481944'),
(0x01a15a9e95f27051b027aa97a9faa197, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'login.success', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '{}', '2026-10-20 18:41:44.947463'),
(0x01a173432842742ba211d26ea0814cd5, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 'refresh.reuse_detected', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '{\"revoked_sessions\": 1}', '2026-10-25 13:32:23.634756'),
(0x01a17343477c7520b137f92e015fcf00, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'login.success', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '{}', '2026-10-25 13:32:31.487253'),
(0x01a173c287b07096bc98b26b9df29b2c, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 'refresh.reuse_detected', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '{\"revoked_sessions\": 1}', '2026-10-25 15:51:30.996891'),
(0x01a173c2bd6d74e6a6364e675adc06a0, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 'login.success', NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '{}', '2026-10-25 15:51:44.752120');

--
-- Triggers `auth_events`
--
DELIMITER $$
CREATE TRIGGER `trg_auth_events_ao_del` BEFORE DELETE ON `auth_events` FOR EACH ROW BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_auth_events_ao_upd` BEFORE UPDATE ON `auth_events` FOR EACH ROW BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `bank_reconciliations`
--

CREATE TABLE `bank_reconciliations` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `account_id` binary(16) NOT NULL,
  `statement_date` date NOT NULL,
  `statement_balance` decimal(18,2) NOT NULL,
  `opening_balance` decimal(18,2) NOT NULL,
  `cleared_balance` decimal(18,2) DEFAULT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'in_progress',
  `note` varchar(500) DEFAULT NULL,
  `created_by` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `completed_at` datetime(6) DEFAULT NULL,
  `completed_by` binary(16) DEFAULT NULL,
  `version` int NOT NULL DEFAULT '1',
  `open_account_id` binary(16) GENERATED ALWAYS AS (if((`status` = _utf8mb4'in_progress'),`account_id`,NULL)) STORED
) ;

--
-- Triggers `bank_reconciliations`
--
DELIMITER $$
CREATE TRIGGER `trg_bank_reconciliations_tg_del` BEFORE DELETE ON `bank_reconciliations` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_bank_reconciliations_tg_ins` BEFORE INSERT ON `bank_reconciliations` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_bank_reconciliations_tg_upd` BEFORE UPDATE ON `bank_reconciliations` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `bank_reconciliation_items`
--

CREATE TABLE `bank_reconciliation_items` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `reconciliation_id` binary(16) NOT NULL,
  `account_transaction_id` binary(16) NOT NULL,
  `amount` decimal(18,2) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Triggers `bank_reconciliation_items`
--
DELIMITER $$
CREATE TRIGGER `trg_bank_reconciliation_items_tg_del` BEFORE DELETE ON `bank_reconciliation_items` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_bank_reconciliation_items_tg_ins` BEFORE INSERT ON `bank_reconciliation_items` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_bank_reconciliation_items_tg_upd` BEFORE UPDATE ON `bank_reconciliation_items` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `branches`
--

CREATE TABLE `branches` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `code` varchar(10) NOT NULL,
  `name` varchar(150) NOT NULL,
  `address` varchar(500) DEFAULT NULL,
  `phone` varchar(20) DEFAULT NULL,
  `is_default` tinyint(1) NOT NULL DEFAULT '0',
  `default_key` tinyint GENERATED ALWAYS AS (if(`is_default`,1,NULL)) VIRTUAL,
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `default_location_id` binary(16) DEFAULT NULL,
  `receipt_header` varchar(500) DEFAULT NULL,
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL
) ;

--
-- Dumping data for table `branches`
--

INSERT INTO `branches` (`tenant_id`, `id`, `code`, `name`, `address`, `phone`, `is_default`, `status`, `default_location_id`, `receipt_header`, `version`, `created_at`, `updated_at`, `created_by`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff30876da9be77f7076393059, 'MAIN', 'Main Branch', NULL, '+8801700000001', 1, 'active', 0x01a14c8ff30c75c6b3bef99b21380151, NULL, 1, '2026-10-18 01:11:04.714726', '2026-10-18 01:11:04.722052', 0x01a14c8ff09475ea92a85fc5af151c9e);

--
-- Triggers `branches`
--
DELIMITER $$
CREATE TRIGGER `trg_branches_tg_del` BEFORE DELETE ON `branches` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_branches_tg_ins` BEFORE INSERT ON `branches` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_branches_tg_upd` BEFORE UPDATE ON `branches` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `brands`
--

CREATE TABLE `brands` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `name` varchar(100) NOT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `deleted_at` datetime(6) DEFAULT NULL,
  `live_name` varchar(100) GENERATED ALWAYS AS (if((`deleted_at` is null),`name`,NULL)) VIRTUAL
) ;

--
-- Dumping data for table `brands`
--

INSERT INTO `brands` (`tenant_id`, `id`, `name`, `status`, `created_at`, `updated_at`, `deleted_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173da52d976e4837a27669b51c389, 'Fresh', 'active', '2026-10-25 16:17:30.331382', '2026-10-25 16:17:30.331382', NULL);

--
-- Triggers `brands`
--
DELIMITER $$
CREATE TRIGGER `trg_brands_tg_del` BEFORE DELETE ON `brands` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_brands_tg_ins` BEFORE INSERT ON `brands` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_brands_tg_upd` BEFORE UPDATE ON `brands` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `cash_registers`
--

CREATE TABLE `cash_registers` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `name` varchar(50) NOT NULL,
  `cash_account_id` binary(16) NOT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL
) ;

--
-- Dumping data for table `cash_registers`
--

INSERT INTO `cash_registers` (`tenant_id`, `id`, `branch_id`, `name`, `cash_account_id`, `status`, `created_at`, `created_by`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a17386776f7022a112740b2ae9f46a, 0x01a14c8ff30876da9be77f7076393059, 'Hello Test Register', 0x11f1cab9ed51475aab00bc542fed17aa, 'active', '2026-10-25 14:45:54.671895', 0x01a14c8ff09475ea92a85fc5af151c9e);

--
-- Triggers `cash_registers`
--
DELIMITER $$
CREATE TRIGGER `trg_cash_registers_tg_del` BEFORE DELETE ON `cash_registers` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_cash_registers_tg_ins` BEFORE INSERT ON `cash_registers` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_cash_registers_tg_upd` BEFORE UPDATE ON `cash_registers` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `categories`
--

CREATE TABLE `categories` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `parent_id` binary(16) DEFAULT NULL,
  `parent_key` binary(16) GENERATED ALWAYS AS (coalesce(`parent_id`,0x00000000000000000000000000000000)) VIRTUAL,
  `name` varchar(100) NOT NULL,
  `depth` tinyint NOT NULL DEFAULT '0',
  `sort_order` int NOT NULL DEFAULT '0',
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `deleted_at` datetime(6) DEFAULT NULL,
  `live_name` varchar(100) GENERATED ALWAYS AS (if((`deleted_at` is null),`name`,NULL)) VIRTUAL
) ;

--
-- Dumping data for table `categories`
--

INSERT INTO `categories` (`tenant_id`, `id`, `parent_id`, `name`, `depth`, `sort_order`, `status`, `version`, `created_at`, `updated_at`, `deleted_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173da393271caa334171028125df8, NULL, 'Grocery', 0, 0, 'active', 1, '2026-10-25 16:17:23.764395', '2026-10-25 16:17:23.764395', NULL);

--
-- Triggers `categories`
--
DELIMITER $$
CREATE TRIGGER `trg_categories_tg_del` BEFORE DELETE ON `categories` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_categories_tg_ins` BEFORE INSERT ON `categories` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_categories_tg_upd` BEFORE UPDATE ON `categories` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `customers`
--

CREATE TABLE `customers` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `code` varchar(20) NOT NULL,
  `name` varchar(150) NOT NULL,
  `phone` varchar(20) DEFAULT NULL,
  `email` varchar(254) DEFAULT NULL,
  `address` varchar(500) DEFAULT NULL,
  `customer_group_id` binary(16) DEFAULT NULL,
  `credit_limit` decimal(18,2) DEFAULT NULL,
  `opening_balance` decimal(18,2) NOT NULL DEFAULT '0.00',
  `current_balance` decimal(18,2) NOT NULL DEFAULT '0.00',
  `is_walk_in` tinyint(1) NOT NULL DEFAULT '0',
  `notes` text,
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `is_sample` tinyint(1) NOT NULL DEFAULT '0',
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL,
  `deleted_at` datetime(6) DEFAULT NULL,
  `live_phone` varchar(20) GENERATED ALWAYS AS (if((`deleted_at` is null),`phone`,NULL)) VIRTUAL,
  `walk_in_key` tinyint GENERATED ALWAYS AS (if(`is_walk_in`,1,NULL)) VIRTUAL
) ;

--
-- Dumping data for table `customers`
--

INSERT INTO `customers` (`tenant_id`, `id`, `code`, `name`, `phone`, `email`, `address`, `customer_group_id`, `credit_limit`, `opening_balance`, `current_balance`, `is_walk_in`, `notes`, `status`, `is_sample`, `version`, `created_at`, `updated_at`, `created_by`, `deleted_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a17389c81c76ec9bb583d2246c37ba, 'C000001', 'Jashi', NULL, NULL, NULL, NULL, NULL, '0.00', '0.00', 0, NULL, 'active', 0, 1, '2026-10-25 14:49:31.934625', '2026-10-25 14:49:31.934625', 0x01a14c8ff09475ea92a85fc5af151c9e, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1738a084873a59ddeb03f71ea2ae5, 'C000002', 'Jashim', '01839707645', NULL, NULL, NULL, NULL, '0.00', '0.00', 0, NULL, 'active', 0, 1, '2026-10-25 14:49:48.361303', '2026-10-25 14:49:48.361303', 0x01a14c8ff09475ea92a85fc5af151c9e, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9efc2e801ab00bc542fed17aa, 'WALKIN', 'Walk-in Customer', NULL, NULL, NULL, NULL, NULL, '0.00', '0.00', 1, NULL, 'active', 0, 1, '2026-10-18 06:05:36.538798', '2026-10-18 06:05:36.538798', NULL, NULL);

--
-- Triggers `customers`
--
DELIMITER $$
CREATE TRIGGER `trg_customers_tg_del` BEFORE DELETE ON `customers` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_customers_tg_ins` BEFORE INSERT ON `customers` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_customers_tg_upd` BEFORE UPDATE ON `customers` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `document_sequences`
--

CREATE TABLE `document_sequences` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) DEFAULT NULL,
  `branch_key` binary(16) GENERATED ALWAYS AS (coalesce(`branch_id`,0x00000000000000000000000000000000)) VIRTUAL,
  `doc_type` varchar(30) NOT NULL,
  `prefix` varchar(10) NOT NULL,
  `include_period` tinyint(1) NOT NULL DEFAULT '1',
  `reset_period` varchar(10) NOT NULL DEFAULT 'never',
  `period_key` varchar(7) DEFAULT NULL,
  `padding` tinyint NOT NULL DEFAULT '5',
  `next_value` bigint NOT NULL DEFAULT '1',
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `document_sequences`
--

INSERT INTO `document_sequences` (`tenant_id`, `id`, `branch_id`, `doc_type`, `prefix`, `include_period`, `reset_period`, `period_key`, `padding`, `next_value`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff39d759fa47e813a4b5c60aa, 0x01a14c8ff30876da9be77f7076393059, 'sale', 'INV', 1, 'never', NULL, 5, 9, '2026-10-25 16:07:06.774292'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3a2763fb68071b80686d9f9, 0x01a14c8ff30876da9be77f7076393059, 'sale_return', 'RET', 1, 'never', NULL, 5, 2, '2026-10-19 02:59:59.771233'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3a470c0bd5d450fb67b217d, 0x01a14c8ff30876da9be77f7076393059, 'purchase', 'PUR', 1, 'never', NULL, 5, 2, '2026-10-25 16:26:33.440494'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3a5709da563d79fa495c19b, 0x01a14c8ff30876da9be77f7076393059, 'goods_receipt', 'GRN', 1, 'never', NULL, 5, 2, '2026-10-25 16:26:33.444312'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3a7724db9923fec8a716e06, 0x01a14c8ff30876da9be77f7076393059, 'payment_in', 'RCV', 1, 'never', NULL, 5, 10, '2026-10-25 16:07:06.779281'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3a9759e9568c32a7e52d6e4, 0x01a14c8ff30876da9be77f7076393059, 'payment_out', 'PMT', 1, 'never', NULL, 5, 5, '2026-10-25 16:26:47.618495'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3ac7428b1e57198507fd04d, 0x01a14c8ff30876da9be77f7076393059, 'expense', 'EXP', 1, 'never', NULL, 5, 2, '2026-10-19 01:47:50.473076'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3ae70dc878ddaff197a2f4e, 0x01a14c8ff30876da9be77f7076393059, 'adjustment', 'ADJ', 1, 'never', NULL, 5, 1, '2026-10-18 01:11:04.879297'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3b07210bbf63aad66bf40ee, 0x01a14c8ff30876da9be77f7076393059, 'transfer', 'TRF', 1, 'never', NULL, 5, 3, '2026-10-25 13:44:30.570629'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3b27089a413a5c1af33e6e0, NULL, 'customer', 'C', 0, 'never', NULL, 6, 3, '2026-10-25 14:49:48.359651'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3b3739cb6911fd4daa48c8d, NULL, 'supplier', 'S', 0, 'never', NULL, 6, 2, '2026-10-19 01:46:17.737399'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3b573d19267a8d6117e883e, NULL, 'product', 'P', 0, 'never', NULL, 6, 1, '2026-10-18 01:11:04.886153'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9f8e6553fab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'purchase_return', 'PRT', 1, 'never', NULL, 5, 1, '2026-10-18 06:05:51.872783'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdebb600ab00bc542fed17aa, NULL, 'journal', 'JV', 1, 'never', NULL, 5, 18, '2026-10-25 16:26:47.635964'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cbe90bf7421cab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'count', 'CNT', 1, 'never', NULL, 5, 1, '2026-10-19 18:15:21.196646'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cbe90bf7460aab00bc542fed17aa, 0x01a14c8ff30876da9be77f7076393059, 'income', 'INC', 1, 'never', NULL, 5, 2, '2026-10-25 13:44:09.486665');

--
-- Triggers `document_sequences`
--
DELIMITER $$
CREATE TRIGGER `trg_document_sequences_tg_del` BEFORE DELETE ON `document_sequences` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_document_sequences_tg_ins` BEFORE INSERT ON `document_sequences` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_document_sequences_tg_upd` BEFORE UPDATE ON `document_sequences` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `expenses`
--

CREATE TABLE `expenses` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `expense_no` varchar(40) NOT NULL,
  `category_id` binary(16) NOT NULL,
  `business_date` date NOT NULL,
  `amount` decimal(18,2) NOT NULL,
  `tax_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `account_id` binary(16) NOT NULL,
  `payment_id` binary(16) DEFAULT NULL,
  `payee` varchar(150) DEFAULT NULL,
  `reference` varchar(100) DEFAULT NULL,
  `note` varchar(500) DEFAULT NULL,
  `attachment_file_id` binary(16) DEFAULT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'posted',
  `approved_by` binary(16) DEFAULT NULL,
  `recurring_rule_id` binary(16) DEFAULT NULL,
  `idempotency_key` binary(16) DEFAULT NULL,
  `voided_at` datetime(6) DEFAULT NULL,
  `voided_by` binary(16) DEFAULT NULL,
  `void_reason` varchar(500) DEFAULT NULL,
  `created_by` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `expenses`
--

INSERT INTO `expenses` (`tenant_id`, `id`, `branch_id`, `expense_no`, `category_id`, `business_date`, `amount`, `tax_amount`, `account_id`, `payment_id`, `payee`, `reference`, `note`, `attachment_file_id`, `status`, `approved_by`, `recurring_rule_id`, `idempotency_key`, `voided_at`, `voided_by`, `void_reason`, `created_by`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d7f749779b8799b76ffb0c0d82, 0x01a14c8ff30876da9be77f7076393059, 'EXP-2610-00001', 0x11f1cab9f743cee0ab00bc542fed17aa, '2026-10-19', '1.00', '0.00', 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a151d7f74d7268837546ff0cb78b63, NULL, NULL, NULL, NULL, 'posted', NULL, NULL, 0x33a8108eaae94cb3a921dfcab13e4d55, NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:47:50.474122');

--
-- Triggers `expenses`
--
DELIMITER $$
CREATE TRIGGER `trg_expenses_tg_del` BEFORE DELETE ON `expenses` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_expenses_tg_ins` BEFORE INSERT ON `expenses` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_expenses_tg_upd` BEFORE UPDATE ON `expenses` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `expense_categories`
--

CREATE TABLE `expense_categories` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `name` varchar(100) NOT NULL,
  `ledger_account_id` binary(16) DEFAULT NULL,
  `is_system` tinyint(1) NOT NULL DEFAULT '0',
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `expense_categories`
--

INSERT INTO `expense_categories` (`tenant_id`, `id`, `name`, `ledger_account_id`, `is_system`, `status`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9f743b805ab00bc542fed17aa, 'Rent', 0x11f1cab9fdb0ba14ab00bc542fed17aa, 1, 'active', '2026-10-18 06:05:49.128255'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9f743c14cab00bc542fed17aa, 'Utilities', 0x11f1cab9fdb0bd53ab00bc542fed17aa, 1, 'active', '2026-10-18 06:05:49.128255'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9f743c5b4ab00bc542fed17aa, 'Salaries', 0x11f1cab9fdb0c072ab00bc542fed17aa, 1, 'active', '2026-10-18 06:05:49.128255'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9f743caebab00bc542fed17aa, 'Transport', 0x11f1cab9fdb0c39aab00bc542fed17aa, 1, 'active', '2026-10-18 06:05:49.128255'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9f743cee0ab00bc542fed17aa, 'Marketing', 0x11f1cab9fdb0c6d1ab00bc542fed17aa, 1, 'active', '2026-10-18 06:05:49.128255'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9f743d272ab00bc542fed17aa, 'Maintenance', 0x11f1cab9fdb0c9e2ab00bc542fed17aa, 1, 'active', '2026-10-18 06:05:49.128255'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9f743d5feab00bc542fed17aa, 'Bank & MFS Charges', 0x11f1cab9fdb0cce7ab00bc542fed17aa, 1, 'active', '2026-10-18 06:05:49.128255'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9f743d831ab00bc542fed17aa, 'Miscellaneous', 0x11f1cab9fdb0d01fab00bc542fed17aa, 1, 'active', '2026-10-18 06:05:49.128255');

--
-- Triggers `expense_categories`
--
DELIMITER $$
CREATE TRIGGER `trg_expense_categories_tg_del` BEFORE DELETE ON `expense_categories` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_expense_categories_tg_ins` BEFORE INSERT ON `expense_categories` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_expense_categories_tg_upd` BEFORE UPDATE ON `expense_categories` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `features`
--

CREATE TABLE `features` (
  `code` varchar(60) NOT NULL,
  `name` varchar(100) NOT NULL,
  `kind` varchar(10) NOT NULL,
  `description` text,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `features`
--

INSERT INTO `features` (`code`, `name`, `kind`, `description`, `created_at`) VALUES
('accounting.double_entry', 'Double-entry accounting', 'flag', 'Chart of accounts and journals (R2)', '2026-10-18 01:05:38.395136'),
('api.access', 'Public API', 'flag', 'API keys and webhooks (R3)', '2026-10-18 01:05:38.395136'),
('branches.max', 'Branches', 'limit', 'Maximum active branches', '2026-10-18 01:05:38.395136'),
('inventory.adjustments', 'Stock adjustments', 'flag', 'Stock adjustment documents', '2026-10-18 01:05:38.395136'),
('inventory.transfers', 'Stock transfers', 'flag', 'Transfers between locations (R2)', '2026-10-18 01:05:38.395136'),
('invoices.monthly_max', 'Invoices per month', 'limit', 'Maximum sales invoices per calendar month', '2026-10-18 01:05:38.395136'),
('notifications.sms', 'SMS notifications', 'flag', 'SMS channel (R2)', '2026-10-18 01:05:38.395136'),
('pos.core', 'POS', 'flag', 'Point of sale', '2026-10-18 01:05:38.395136'),
('products.max', 'Products', 'limit', 'Maximum active product variants', '2026-10-18 01:05:38.395136'),
('products.variants', 'Product variants', 'flag', 'Variant matrix UI (R2)', '2026-10-18 01:05:38.395136'),
('purchases.orders', 'Purchase orders', 'flag', 'Purchase orders and approval (R2)', '2026-10-18 01:05:38.395136'),
('registers.max', 'Cash registers', 'limit', 'Maximum cash registers (R2)', '2026-10-18 01:05:38.395136'),
('reports.advanced', 'Advanced reports', 'flag', 'Aging, VAT, movers (R2)', '2026-10-18 01:05:38.395136'),
('reports.export', 'Report export', 'flag', 'CSV/XLSX/PDF export', '2026-10-18 01:05:38.395136'),
('storage.mb', 'File storage (MB)', 'limit', 'Object storage quota', '2026-10-18 01:05:38.395136'),
('users.max', 'Users', 'limit', 'Maximum active members', '2026-10-18 01:05:38.395136'),
('whitelabel', 'White label', 'flag', 'Custom branding and domain (R4)', '2026-10-18 01:05:38.395136');

-- --------------------------------------------------------

--
-- Table structure for table `files`
--

CREATE TABLE `files` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `storage_key` varchar(300) NOT NULL,
  `file_name` varchar(255) NOT NULL,
  `content_type` varchar(100) NOT NULL,
  `size_bytes` bigint NOT NULL,
  `checksum_sha256` binary(32) DEFAULT NULL,
  `purpose` varchar(20) NOT NULL,
  `entity_type` varchar(40) DEFAULT NULL,
  `entity_id` binary(16) DEFAULT NULL,
  `uploaded_by` binary(16) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `expires_at` datetime(6) DEFAULT NULL
) ;

--
-- Triggers `files`
--
DELIMITER $$
CREATE TRIGGER `trg_files_tg_del` BEFORE DELETE ON `files` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_files_tg_ins` BEFORE INSERT ON `files` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_files_tg_upd` BEFORE UPDATE ON `files` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `fiscal_periods`
--

CREATE TABLE `fiscal_periods` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `start_date` date NOT NULL,
  `end_date` date NOT NULL,
  `status` varchar(10) NOT NULL DEFAULT 'open',
  `locked_at` datetime(6) DEFAULT NULL,
  `locked_by` binary(16) DEFAULT NULL
) ;

--
-- Triggers `fiscal_periods`
--
DELIMITER $$
CREATE TRIGGER `trg_fiscal_periods_tg_del` BEFORE DELETE ON `fiscal_periods` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_fiscal_periods_tg_ins` BEFORE INSERT ON `fiscal_periods` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_fiscal_periods_tg_upd` BEFORE UPDATE ON `fiscal_periods` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `goods_receipts`
--

CREATE TABLE `goods_receipts` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `receipt_no` varchar(40) NOT NULL,
  `purchase_id` binary(16) NOT NULL,
  `location_id` binary(16) NOT NULL,
  `received_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `business_date` date NOT NULL,
  `note` varchar(500) DEFAULT NULL,
  `received_by` binary(16) NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Dumping data for table `goods_receipts`
--

INSERT INTO `goods_receipts` (`tenant_id`, `id`, `receipt_no`, `purchase_id`, `location_id`, `received_at`, `business_date`, `note`, `received_by`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e29c65768cb83fc880e0178154, 'GRN-2610-00001', 0x01a173e29933711bbff0746df7a0951b, 0x01a14c8ff30c75c6b3bef99b21380151, '2026-10-25 16:26:33.797902', '2026-10-25', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e);

--
-- Triggers `goods_receipts`
--
DELIMITER $$
CREATE TRIGGER `trg_goods_receipts_tg_del` BEFORE DELETE ON `goods_receipts` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_goods_receipts_tg_ins` BEFORE INSERT ON `goods_receipts` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_goods_receipts_tg_upd` BEFORE UPDATE ON `goods_receipts` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `goods_receipt_items`
--

CREATE TABLE `goods_receipt_items` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `receipt_id` binary(16) NOT NULL,
  `purchase_item_id` binary(16) NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `base_quantity` decimal(18,3) NOT NULL,
  `unit_cost` decimal(18,4) NOT NULL,
  `line_value` decimal(18,2) NOT NULL DEFAULT '0.00',
  `tax_amount` decimal(18,2) NOT NULL DEFAULT '0.00'
) ;

--
-- Dumping data for table `goods_receipt_items`
--

INSERT INTO `goods_receipt_items` (`tenant_id`, `id`, `receipt_id`, `purchase_item_id`, `variant_id`, `base_quantity`, `unit_cost`, `line_value`, `tax_amount`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e29dc77730af0d1ce11811effa, 0x01a173e29c65768cb83fc880e0178154, 0x01a173e299b370b593e7634198c2b81c, 0x01a173e1bc2f72d38cbf3f359dbef54a, '1.000', '4.0000', '4.00', '0.00');

--
-- Triggers `goods_receipt_items`
--
DELIMITER $$
CREATE TRIGGER `trg_goods_receipt_items_tg_del` BEFORE DELETE ON `goods_receipt_items` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_goods_receipt_items_tg_ins` BEFORE INSERT ON `goods_receipt_items` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_goods_receipt_items_tg_upd` BEFORE UPDATE ON `goods_receipt_items` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `held_carts`
--

CREATE TABLE `held_carts` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `label` varchar(100) DEFAULT NULL,
  `customer_id` binary(16) DEFAULT NULL,
  `cart` json NOT NULL,
  `held_by` binary(16) NOT NULL,
  `expires_at` datetime(6) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Triggers `held_carts`
--
DELIMITER $$
CREATE TRIGGER `trg_held_carts_tg_del` BEFORE DELETE ON `held_carts` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_held_carts_tg_ins` BEFORE INSERT ON `held_carts` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_held_carts_tg_upd` BEFORE UPDATE ON `held_carts` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `idempotency_keys`
--

CREATE TABLE `idempotency_keys` (
  `tenant_id` binary(16) NOT NULL,
  `idem_key` binary(16) NOT NULL,
  `user_id` binary(16) NOT NULL,
  `endpoint` varchar(100) NOT NULL,
  `request_hash` binary(32) NOT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'in_progress',
  `response_status` smallint DEFAULT NULL,
  `response_body` json DEFAULT NULL,
  `resource_type` varchar(40) DEFAULT NULL,
  `resource_id` binary(16) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `expires_at` datetime(6) NOT NULL
) ;

--
-- Dumping data for table `idempotency_keys`
--

INSERT INTO `idempotency_keys` (`tenant_id`, `idem_key`, `user_id`, `endpoint`, `request_hash`, `status`, `response_status`, `response_body`, `resource_type`, `resource_id`, `created_at`, `expires_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x023565c31702411daefd298ad5815f20, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /supplier-payments', 0x465600eaeecc8247ccb0235aa56af31d813bab482504c1e72a524e28188e597a, 'completed', 201, '{\"id\": \"01a173e2-d3c3-75b8-9173-7cae376653cb\", \"note\": \"Payment to Mr Kashem\", \"amount\": \"4.00\", \"source\": \"supplier_payment\", \"status\": \"posted\", \"party_id\": \"01a151d6-8d09-74f8-bbb2-203ca6b1e0e0\", \"direction\": \"out\", \"reference\": null, \"account_id\": \"11f1cab9-ed51-475a-ab00-bc542fed17aa\", \"created_at\": \"2026-10-25T16:26:47.620177Z\", \"party_name\": \"Mr Kashem\", \"party_type\": \"supplier\", \"payment_no\": \"PMT-2610-00004\", \"allocations\": [{\"amount\": \"4.00\", \"document_id\": \"01a173e2-9933-711b-bff0-746df7a0951b\", \"document_no\": \"PUR-2610-00001\", \"reversed_at\": null, \"document_type\": \"purchase\"}], \"method_name\": \"Cash\", \"void_reason\": null, \"account_name\": \"Cash\", \"payment_date\": \"2026-10-25T16:26:47.620177Z\", \"business_date\": \"2026-10-25\", \"created_by_name\": \"Test Owner\", \"payment_method_id\": \"11f1cab9-ed5d-0607-ab00-bc542fed17aa\", \"unallocated_amount\": \"0.00\"}', 'payment', 0x01a173e2d3c375b891737cae376653cb, '2026-10-25 16:26:47.610124', '2026-10-28 16:26:47.610124'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x04e83fabc6494c538e3d70f3ea1e031b, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /sales', 0x1bc7ef49d29cec5c62051724c33cc4ede602a35e90538d8e9d21f3cae7a025f0, 'completed', 201, '{\"id\": \"01a15a9f-d9e6-747d-b530-70ca2ec2e354\", \"note\": null, \"items\": [{\"id\": \"01a15a9f-d9e8-73ab-a80b-df73d473feca\", \"sku\": \"Ata-001\", \"line_no\": 1, \"quantity\": \"2.000\", \"unit_cost\": \"0.0000\", \"cost_total\": \"0.00\", \"line_total\": \"4.20\", \"list_price\": \"2.0000\", \"tax_amount\": \"0.20\", \"unit_price\": \"2.0000\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"gross_amount\": \"4.00\", \"product_name\": \"Ata 800\", \"tax_rate_pct\": \"5.0000\", \"discount_type\": null, \"line_discount\": \"0.00\", \"discount_value\": null, \"taxable_amount\": \"4.00\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"price_overridden\": false, \"returned_quantity\": \"0.000\", \"invoice_discount_alloc\": \"0.00\"}], \"status\": \"completed\", \"receipt\": {\"phone\": \"+8801700000001\", \"address\": null, \"auto_print\": true, \"bin_number\": null, \"branch_name\": \"Main Branch\", \"business_name\": \"Test Shop\", \"receipt_paper\": \"80mm\", \"receipt_footer\": null, \"receipt_header\": null}, \"returns\": [], \"payments\": [{\"amount\": \"4.00\", \"source\": \"pos\", \"status\": \"posted\", \"reference\": null, \"created_at\": \"2026-10-20T18:43:07.888205Z\", \"payment_id\": \"01a15a9f-d9ef-7012-946c-3bc56619a222\", \"payment_no\": \"RCV-2610-00004\", \"method_name\": \"Cash\", \"account_name\": \"Cash\"}], \"subtotal\": \"4.00\", \"due_total\": \"0.00\", \"sale_date\": \"2026-10-20T18:43:07.871152Z\", \"tax_total\": \"0.20\", \"cashier_id\": \"01a14c8f-f094-75ea-92a8-5fc5af151c9e\", \"cost_total\": \"0.00\", \"created_at\": \"2026-10-20T18:43:07.881674Z\", \"invoice_no\": \"INV-2610-00004\", \"is_walk_in\": true, \"item_count\": 1, \"paid_total\": \"4.00\", \"customer_id\": \"11f1cab9-efc2-e801-ab00-bc542fed17aa\", \"grand_total\": \"4.00\", \"cancelled_at\": null, \"cashier_name\": \"Test Owner\", \"change_given\": \"0.00\", \"other_charge\": \"0.00\", \"business_date\": \"2026-10-21\", \"cancel_reason\": null, \"customer_name\": \"Walk-in Customer\", \"reprint_count\": 0, \"customer_phone\": null, \"payment_status\": \"paid\", \"returned_total\": \"0.00\", \"shipping_charge\": \"0.00\", \"prices_include_tax\": false, \"item_discount_total\": \"0.00\", \"rounding_adjustment\": \"-0.20\", \"invoice_discount_type\": null, \"invoice_discount_value\": null, \"invoice_discount_amount\": \"0.00\"}', 'sale', 0x01a15a9fd9e6747db53070ca2ec2e354, '2026-10-20 18:43:07.869389', '2026-10-23 18:43:07.869389'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x0a042dc0a80244408d9a762844addb67, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /sales', 0x76f19da04c84c4f02705252b94896fdb2fd9b069918134c0caf283af34da3ff3, 'completed', 201, '{\"id\": \"01a14da3-36ca-723d-ac4f-ec5f7d9b1aeb\", \"note\": null, \"items\": [{\"id\": \"01a14da3-36cc-700b-b5e8-3fe7b92860c5\", \"sku\": \"Ata-001\", \"line_no\": 1, \"quantity\": \"3.000\", \"unit_cost\": \"0.0000\", \"cost_total\": \"0.00\", \"line_total\": \"5.67\", \"list_price\": \"2.0000\", \"tax_amount\": \"0.27\", \"unit_price\": \"2.0000\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"gross_amount\": \"6.00\", \"product_name\": \"Ata 800\", \"tax_rate_pct\": \"5.0000\", \"discount_type\": \"percent\", \"line_discount\": \"0.60\", \"discount_value\": \"10.0000\", \"taxable_amount\": \"5.40\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"price_overridden\": false, \"returned_quantity\": \"0.000\", \"invoice_discount_alloc\": \"0.00\"}], \"status\": \"completed\", \"receipt\": {\"phone\": \"+8801700000001\", \"address\": null, \"auto_print\": true, \"bin_number\": null, \"branch_name\": \"Main Branch\", \"business_name\": \"Test Shop\", \"receipt_paper\": \"80mm\", \"receipt_footer\": null, \"receipt_header\": null}, \"returns\": [], \"payments\": [{\"amount\": \"6.00\", \"source\": \"pos\", \"status\": \"posted\", \"reference\": null, \"created_at\": \"2026-10-18T06:11:44.465837Z\", \"payment_id\": \"01a14da3-36d1-71c1-96e4-9427b8d3097a\", \"payment_no\": \"RCV-2610-00002\", \"method_name\": \"Cash\", \"account_name\": \"Cash\"}], \"subtotal\": \"6.00\", \"due_total\": \"0.00\", \"sale_date\": \"2026-10-18T06:11:44.455039Z\", \"tax_total\": \"0.27\", \"cashier_id\": \"01a14c8f-f094-75ea-92a8-5fc5af151c9e\", \"cost_total\": \"0.00\", \"created_at\": \"2026-10-18T06:11:44.461257Z\", \"invoice_no\": \"INV-2610-00002\", \"is_walk_in\": true, \"item_count\": 1, \"paid_total\": \"6.00\", \"customer_id\": \"11f1cab9-efc2-e801-ab00-bc542fed17aa\", \"grand_total\": \"6.00\", \"cancelled_at\": null, \"cashier_name\": \"Test Owner\", \"change_given\": \"0.00\", \"other_charge\": \"0.00\", \"business_date\": \"2026-10-18\", \"cancel_reason\": null, \"customer_name\": \"Walk-in Customer\", \"reprint_count\": 0, \"customer_phone\": null, \"payment_status\": \"paid\", \"returned_total\": \"0.00\", \"shipping_charge\": \"0.00\", \"prices_include_tax\": false, \"item_discount_total\": \"0.60\", \"rounding_adjustment\": \"0.33\", \"invoice_discount_type\": null, \"invoice_discount_value\": null, \"invoice_discount_amount\": \"0.00\"}', 'sale', 0x01a14da336ca723dac4fec5f7d9b1aeb, '2026-10-18 06:11:44.453861', '2026-10-21 06:11:44.453861'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x138543dd78f743819d3014a402285d1d, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /sales', 0x58e041f17082b313ee03388e39adfb6ee5aca8274a475acecf8734def69c7a3b, 'completed', 201, '{\"id\": \"01a173d0-cf15-7236-a453-75e8a5272e85\", \"note\": null, \"items\": [{\"id\": \"01a173d0-cf16-737a-8950-438aa28b2252\", \"sku\": \"Ata-001\", \"line_no\": 1, \"quantity\": \"1.000\", \"unit_cost\": \"0.0000\", \"cost_total\": \"0.00\", \"line_total\": \"2.10\", \"list_price\": \"2.0000\", \"tax_amount\": \"0.10\", \"unit_price\": \"2.0000\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"gross_amount\": \"2.00\", \"product_name\": \"Ata 800\", \"tax_rate_pct\": \"5.0000\", \"discount_type\": null, \"line_discount\": \"0.00\", \"discount_value\": null, \"taxable_amount\": \"2.00\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"price_overridden\": false, \"returned_quantity\": \"0.000\", \"invoice_discount_alloc\": \"0.00\"}], \"status\": \"completed\", \"receipt\": {\"phone\": \"+8801700000001\", \"address\": null, \"auto_print\": true, \"bin_number\": null, \"branch_name\": \"Main Branch\", \"business_name\": \"Test Shop\", \"receipt_paper\": \"80mm\", \"receipt_footer\": null, \"receipt_header\": null}, \"returns\": [], \"payments\": [{\"amount\": \"2.00\", \"source\": \"pos\", \"status\": \"posted\", \"reference\": null, \"created_at\": \"2026-10-25T16:07:06.779935Z\", \"payment_id\": \"01a173d0-cf1b-75a0-a5d1-6d55080ed558\", \"payment_no\": \"RCV-2610-00009\", \"method_name\": \"Cash\", \"account_name\": \"Cash\"}], \"subtotal\": \"2.00\", \"due_total\": \"0.00\", \"sale_date\": \"2026-10-25T16:07:06.770206Z\", \"tax_total\": \"0.10\", \"cashier_id\": \"01a14c8f-f094-75ea-92a8-5fc5af151c9e\", \"cost_total\": \"0.00\", \"created_at\": \"2026-10-25T16:07:06.775122Z\", \"invoice_no\": \"INV-2610-00008\", \"is_walk_in\": true, \"item_count\": 1, \"paid_total\": \"2.00\", \"customer_id\": \"11f1cab9-efc2-e801-ab00-bc542fed17aa\", \"grand_total\": \"2.00\", \"cancelled_at\": null, \"cashier_name\": \"Test Owner\", \"change_given\": \"0.00\", \"other_charge\": \"0.00\", \"business_date\": \"2026-10-25\", \"cancel_reason\": null, \"customer_name\": \"Walk-in Customer\", \"reprint_count\": 0, \"customer_phone\": null, \"payment_status\": \"paid\", \"returned_total\": \"0.00\", \"shipping_charge\": \"0.00\", \"prices_include_tax\": false, \"item_discount_total\": \"0.00\", \"rounding_adjustment\": \"-0.10\", \"invoice_discount_type\": null, \"invoice_discount_value\": null, \"invoice_discount_amount\": \"0.00\"}', 'sale', 0x01a173d0cf157236a45375e8a5272e85, '2026-10-25 16:07:06.768181', '2026-10-28 16:07:06.768181'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x13d6342cf0034bc2a1c8b68da1bf2efb, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /account-transfers', 0xb661f4ed7b7b848320e01d9795b336819b09d7caddc399ce254ffeaa0a369d7d, 'completed', 201, '{\"id\": \"01a1734e-406b-71b8-8753-64222b7b6a82\", \"note\": null, \"amount\": \"1.00\", \"status\": \"posted\", \"reference\": null, \"created_at\": \"2026-10-25T13:44:30.571842Z\", \"fee_amount\": \"0.00\", \"transfer_no\": \"TRF-2610-00002\", \"business_date\": \"2026-10-25\", \"to_account_id\": \"11f1cab9-ed51-55c9-ab00-bc542fed17aa\", \"from_account_id\": \"11f1cab9-ed51-475a-ab00-bc542fed17aa\", \"to_account_name\": \"Bank Account\", \"from_account_name\": \"Cash\"}', 'account_transfer', 0x01a1734e406b71b8875364222b7b6a82, '2026-10-25 13:44:30.510256', '2026-10-28 13:44:30.510256'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x13ef1df8c7884a69a24734d0b4c10d04, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /sales', 0x1bc7ef49d29cec5c62051724c33cc4ede602a35e90538d8e9d21f3cae7a025f0, 'completed', 201, '{\"id\": \"01a151d3-3c73-7151-8021-9c1cedd1bb46\", \"note\": null, \"items\": [{\"id\": \"01a151d3-3c75-74c6-9c99-233c18ca09c6\", \"sku\": \"Ata-001\", \"line_no\": 1, \"quantity\": \"2.000\", \"unit_cost\": \"0.0000\", \"cost_total\": \"0.00\", \"line_total\": \"4.20\", \"list_price\": \"2.0000\", \"tax_amount\": \"0.20\", \"unit_price\": \"2.0000\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"gross_amount\": \"4.00\", \"product_name\": \"Ata 800\", \"tax_rate_pct\": \"5.0000\", \"discount_type\": null, \"line_discount\": \"0.00\", \"discount_value\": null, \"taxable_amount\": \"4.00\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"price_overridden\": false, \"returned_quantity\": \"0.000\", \"invoice_discount_alloc\": \"0.00\"}], \"status\": \"completed\", \"receipt\": {\"phone\": \"+8801700000001\", \"address\": null, \"auto_print\": true, \"bin_number\": null, \"branch_name\": \"Main Branch\", \"business_name\": \"Test Shop\", \"receipt_paper\": \"80mm\", \"receipt_footer\": null, \"receipt_header\": null}, \"returns\": [], \"payments\": [{\"amount\": \"4.00\", \"source\": \"pos\", \"status\": \"posted\", \"reference\": null, \"created_at\": \"2026-10-19T01:42:40.508845Z\", \"payment_id\": \"01a151d3-3c7c-7573-ba30-980ef714912f\", \"payment_no\": \"RCV-2610-00003\", \"method_name\": \"Cash\", \"account_name\": \"Cash\"}], \"subtotal\": \"4.00\", \"due_total\": \"0.00\", \"sale_date\": \"2026-10-19T01:42:40.494213Z\", \"tax_total\": \"0.20\", \"cashier_id\": \"01a14c8f-f094-75ea-92a8-5fc5af151c9e\", \"cost_total\": \"0.00\", \"created_at\": \"2026-10-19T01:42:40.503106Z\", \"invoice_no\": \"INV-2610-00003\", \"is_walk_in\": true, \"item_count\": 1, \"paid_total\": \"4.00\", \"customer_id\": \"11f1cab9-efc2-e801-ab00-bc542fed17aa\", \"grand_total\": \"4.00\", \"cancelled_at\": null, \"cashier_name\": \"Test Owner\", \"change_given\": \"0.00\", \"other_charge\": \"0.00\", \"business_date\": \"2026-10-19\", \"cancel_reason\": null, \"customer_name\": \"Walk-in Customer\", \"reprint_count\": 0, \"customer_phone\": null, \"payment_status\": \"paid\", \"returned_total\": \"0.00\", \"shipping_charge\": \"0.00\", \"prices_include_tax\": false, \"item_discount_total\": \"0.00\", \"rounding_adjustment\": \"-0.20\", \"invoice_discount_type\": null, \"invoice_discount_value\": null, \"invoice_discount_amount\": \"0.00\"}', 'sale', 0x01a151d33c73715180219c1cedd1bb46, '2026-10-19 01:42:40.491535', '2026-10-22 01:42:40.491535'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x15cfdd7ab6cd47019f26458fe4f1c51b, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /supplier-payments', 0x0f79ca5a78dfa5dccf5bcef0f6ed438c8aadfce0bd96fa1abe44ca942ebbed19, 'completed', 201, '{\"id\": \"01a151d7-24e4-7199-b13d-e8df2c8f054e\", \"note\": \"Payment to Mr Kashem\", \"amount\": \"100.00\", \"source\": \"supplier_payment\", \"status\": \"posted\", \"party_id\": \"01a151d6-8d09-74f8-bbb2-203ca6b1e0e0\", \"direction\": \"out\", \"reference\": \"5\", \"account_id\": \"11f1cab9-ed51-475a-ab00-bc542fed17aa\", \"created_at\": \"2026-10-19T01:46:56.613207Z\", \"party_name\": \"Mr Kashem\", \"party_type\": \"supplier\", \"payment_no\": \"PMT-2610-00001\", \"allocations\": [], \"method_name\": \"Cash\", \"void_reason\": null, \"account_name\": \"Cash\", \"payment_date\": \"2026-10-19T01:46:56.613207Z\", \"business_date\": \"2026-10-19\", \"created_by_name\": \"Test Owner\", \"payment_method_id\": \"11f1cab9-ed5d-0607-ab00-bc542fed17aa\", \"unallocated_amount\": \"100.00\"}', 'payment', 0x01a151d724e47199b13de8df2c8f054e, '2026-10-19 01:46:56.607938', '2026-10-22 01:46:56.607938'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x24d8d1b1e499481f9aa09274d3c89c01, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /sales', 0x22d4600f0f65ca66df9f83435f1d4bc4f29017377e01f39fa8b727e7de8fae15, 'completed', 201, '{\"id\": \"01a173cb-7caa-7227-ac4c-13cf122599af\", \"note\": null, \"items\": [{\"id\": \"01a173cb-7cab-75b1-972c-7ef148471b8e\", \"sku\": \"Ata-001\", \"line_no\": 1, \"quantity\": \"1.000\", \"unit_cost\": \"0.0000\", \"cost_total\": \"0.00\", \"line_total\": \"2.10\", \"list_price\": \"2.0000\", \"tax_amount\": \"0.10\", \"unit_price\": \"2.0000\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"gross_amount\": \"2.00\", \"product_name\": \"Ata 800\", \"tax_rate_pct\": \"5.0000\", \"discount_type\": null, \"line_discount\": \"0.00\", \"discount_value\": null, \"taxable_amount\": \"2.00\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"price_overridden\": false, \"returned_quantity\": \"0.000\", \"invoice_discount_alloc\": \"0.00\"}], \"status\": \"completed\", \"receipt\": {\"phone\": \"+8801700000001\", \"address\": null, \"auto_print\": true, \"bin_number\": null, \"branch_name\": \"Main Branch\", \"business_name\": \"Test Shop\", \"receipt_paper\": \"80mm\", \"receipt_footer\": null, \"receipt_header\": null}, \"returns\": [], \"payments\": [{\"amount\": \"2.00\", \"source\": \"pos\", \"status\": \"posted\", \"reference\": null, \"created_at\": \"2026-10-25T16:01:18.002917Z\", \"payment_id\": \"01a173cb-7cb2-7077-9f68-efc75313eb1b\", \"payment_no\": \"RCV-2610-00007\", \"method_name\": \"Cash\", \"account_name\": \"Cash\"}], \"subtotal\": \"2.00\", \"due_total\": \"0.00\", \"sale_date\": \"2026-10-25T16:01:17.990243Z\", \"tax_total\": \"0.10\", \"cashier_id\": \"01a14c8f-f094-75ea-92a8-5fc5af151c9e\", \"cost_total\": \"0.00\", \"created_at\": \"2026-10-25T16:01:17.996594Z\", \"invoice_no\": \"INV-2610-00006\", \"is_walk_in\": true, \"item_count\": 1, \"paid_total\": \"2.00\", \"customer_id\": \"11f1cab9-efc2-e801-ab00-bc542fed17aa\", \"grand_total\": \"2.00\", \"cancelled_at\": null, \"cashier_name\": \"Test Owner\", \"change_given\": \"8.00\", \"other_charge\": \"0.00\", \"business_date\": \"2026-10-25\", \"cancel_reason\": null, \"customer_name\": \"Walk-in Customer\", \"reprint_count\": 0, \"customer_phone\": null, \"payment_status\": \"paid\", \"returned_total\": \"0.00\", \"shipping_charge\": \"0.00\", \"prices_include_tax\": false, \"item_discount_total\": \"0.00\", \"rounding_adjustment\": \"-0.10\", \"invoice_discount_type\": null, \"invoice_discount_value\": null, \"invoice_discount_amount\": \"0.00\"}', 'sale', 0x01a173cb7caa7227ac4c13cf122599af, '2026-10-25 16:01:17.989045', '2026-10-28 16:01:17.989045'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x33a8108eaae94cb3a921dfcab13e4d55, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /expenses', 0xb4dca6610fb5d82e0d956fdb2db4e702bd70c8e162d82f8bafad44f82603b94f, 'completed', 201, '{\"id\": \"01a151d7-f749-779b-8799-b76ffb0c0d82\", \"note\": null, \"payee\": null, \"amount\": \"1.00\", \"status\": \"posted\", \"reference\": null, \"account_id\": \"11f1cab9-ed51-475a-ab00-bc542fed17aa\", \"created_at\": \"2026-10-19T01:47:50.474122Z\", \"expense_no\": \"EXP-2610-00001\", \"payment_id\": \"01a151d7-f74d-7268-8375-46ff0cb78b63\", \"tax_amount\": \"0.00\", \"category_id\": \"11f1cab9-f743-cee0-ab00-bc542fed17aa\", \"void_reason\": null, \"account_name\": \"Cash\", \"business_date\": \"2026-10-19\", \"category_name\": \"Marketing\", \"created_by_name\": \"Test Owner\"}', 'expense', 0x01a151d7f749779b8799b76ffb0c0d82, '2026-10-19 01:47:50.464230', '2026-10-22 01:47:50.464230'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x4e83bbc9f674402695ee447020ee8433, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /account-transfers', 0xaefe05a3b36ab09bbb064149308aa1e1733a5ac0129469330d379665204f74a1, 'completed', 201, '{\"id\": \"01a15563-7515-710e-9024-7d06ca9ef0ad\", \"note\": \"t\", \"amount\": \"10.00\", \"status\": \"posted\", \"reference\": \"test\", \"created_at\": \"2026-10-19T18:19:03.831543Z\", \"fee_amount\": \"0.00\", \"transfer_no\": \"TRF-2610-00001\", \"business_date\": \"2026-10-20\", \"to_account_id\": \"11f1cab9-ed51-5b95-ab00-bc542fed17aa\", \"from_account_id\": \"11f1cab9-ed51-475a-ab00-bc542fed17aa\", \"to_account_name\": \"Mobile Wallet\", \"from_account_name\": \"Cash\"}', 'account_transfer', 0x01a155637515710e90247d06ca9ef0ad, '2026-10-19 18:19:03.759250', '2026-10-22 18:19:03.759250'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x7a469dae9ac34ae2adb691674878c177, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /sales', 0x22d4600f0f65ca66df9f83435f1d4bc4f29017377e01f39fa8b727e7de8fae15, 'completed', 201, '{\"id\": \"01a173c3-c7a4-76a4-8a17-e9cd82ddd428\", \"note\": null, \"items\": [{\"id\": \"01a173c3-c7ad-708a-b74b-1eb63be36d68\", \"sku\": \"Ata-001\", \"line_no\": 1, \"quantity\": \"1.000\", \"unit_cost\": \"0.0000\", \"cost_total\": \"0.00\", \"line_total\": \"2.10\", \"list_price\": \"2.0000\", \"tax_amount\": \"0.10\", \"unit_price\": \"2.0000\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"gross_amount\": \"2.00\", \"product_name\": \"Ata 800\", \"tax_rate_pct\": \"5.0000\", \"discount_type\": null, \"line_discount\": \"0.00\", \"discount_value\": null, \"taxable_amount\": \"2.00\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"price_overridden\": false, \"returned_quantity\": \"0.000\", \"invoice_discount_alloc\": \"0.00\"}], \"status\": \"completed\", \"receipt\": {\"phone\": \"+8801700000001\", \"address\": null, \"auto_print\": true, \"bin_number\": null, \"branch_name\": \"Main Branch\", \"business_name\": \"Test Shop\", \"receipt_paper\": \"80mm\", \"receipt_footer\": null, \"receipt_header\": null}, \"returns\": [], \"payments\": [{\"amount\": \"2.00\", \"source\": \"pos\", \"status\": \"posted\", \"reference\": null, \"created_at\": \"2026-10-25T15:52:53.054563Z\", \"payment_id\": \"01a173c3-c83b-77c6-9872-17e24cc6448b\", \"payment_no\": \"RCV-2610-00006\", \"method_name\": \"Cash\", \"account_name\": \"Cash\"}], \"subtotal\": \"2.00\", \"due_total\": \"0.00\", \"sale_date\": \"2026-10-25T15:52:52.887641Z\", \"tax_total\": \"0.10\", \"cashier_id\": \"01a14c8f-f094-75ea-92a8-5fc5af151c9e\", \"cost_total\": \"0.00\", \"created_at\": \"2026-10-25T15:52:52.913063Z\", \"invoice_no\": \"INV-2610-00005\", \"is_walk_in\": true, \"item_count\": 1, \"paid_total\": \"2.00\", \"customer_id\": \"11f1cab9-efc2-e801-ab00-bc542fed17aa\", \"grand_total\": \"2.00\", \"cancelled_at\": null, \"cashier_name\": \"Test Owner\", \"change_given\": \"8.00\", \"other_charge\": \"0.00\", \"business_date\": \"2026-10-25\", \"cancel_reason\": null, \"customer_name\": \"Walk-in Customer\", \"reprint_count\": 0, \"customer_phone\": null, \"payment_status\": \"paid\", \"returned_total\": \"0.00\", \"shipping_charge\": \"0.00\", \"prices_include_tax\": false, \"item_discount_total\": \"0.00\", \"rounding_adjustment\": \"-0.10\", \"invoice_discount_type\": null, \"invoice_discount_value\": null, \"invoice_discount_amount\": \"0.00\"}', 'sale', 0x01a173c3c7a476a48a17e9cd82ddd428, '2026-10-25 15:52:52.790498', '2026-10-28 15:52:52.790498'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0xb75b5fdc9589401ba6bad46ddeba1a6a, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /sale-returns', 0x5815bf6d040be1d334e4f13fd5169e0ca19e498ae6dd22dcbed0941ce24eb4f5, 'completed', 201, '{\"id\": \"01a1521a-0698-74d2-a519-0fdfcd9f0412\", \"items\": [{\"id\": \"01a1521a-069b-77bc-8726-2db83d102c2c\", \"sku\": \"Ata-001\", \"quantity\": \"1.000\", \"line_total\": \"2.10\", \"tax_amount\": \"0.10\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"product_name\": \"Ata 800\", \"sale_item_id\": \"01a151d3-3c75-74c6-9c99-233c18ca09c6\", \"item_condition\": \"restock\", \"unit_short_name\": \"kg\"}], \"total\": \"2.10\", \"reason\": \"test\", \"receipt\": {\"phone\": \"+8801700000001\", \"address\": null, \"auto_print\": true, \"bin_number\": null, \"branch_name\": \"Main Branch\", \"business_name\": \"Test Shop\", \"receipt_paper\": \"80mm\", \"receipt_footer\": null, \"receipt_header\": null}, \"sale_id\": \"01a151d3-3c73-7151-8021-9c1cedd1bb46\", \"subtotal\": \"2.00\", \"return_no\": \"RET-2610-00001\", \"tax_total\": \"0.10\", \"cost_total\": \"0.00\", \"created_at\": \"2026-10-19T02:59:59.772675Z\", \"invoice_no\": \"INV-2610-00003\", \"customer_id\": \"11f1cab9-efc2-e801-ab00-bc542fed17aa\", \"refund_mode\": \"cash\", \"business_date\": \"2026-10-19\", \"customer_name\": \"Walk-in Customer\", \"applied_to_due\": \"0.00\", \"created_by_name\": \"Test Owner\", \"credited_amount\": \"0.00\", \"refunded_amount\": \"2.10\"}', 'sale_return', 0x01a1521a069874d2a5190fdfcd9f0412, '2026-10-19 02:59:59.761882', '2026-10-22 02:59:59.761882'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0xca51c7a57ada403e9321cc7bdbab9567, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /sales', 0x58e041f17082b313ee03388e39adfb6ee5aca8274a475acecf8734def69c7a3b, 'completed', 201, '{\"id\": \"01a14d9f-f28c-702f-8002-84db35995a81\", \"note\": null, \"items\": [{\"id\": \"01a14d9f-f28e-7600-b632-f67be7c07aad\", \"sku\": \"Ata-001\", \"line_no\": 1, \"quantity\": \"1.000\", \"unit_cost\": \"0.0000\", \"cost_total\": \"0.00\", \"line_total\": \"2.10\", \"list_price\": \"2.0000\", \"tax_amount\": \"0.10\", \"unit_price\": \"2.0000\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"gross_amount\": \"2.00\", \"product_name\": \"Ata 800\", \"tax_rate_pct\": \"5.0000\", \"discount_type\": null, \"line_discount\": \"0.00\", \"discount_value\": null, \"taxable_amount\": \"2.00\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"price_overridden\": false, \"returned_quantity\": \"0.000\", \"invoice_discount_alloc\": \"0.00\"}], \"status\": \"completed\", \"receipt\": {\"phone\": \"+8801700000001\", \"address\": null, \"auto_print\": true, \"bin_number\": null, \"branch_name\": \"Main Branch\", \"business_name\": \"Test Shop\", \"receipt_paper\": \"80mm\", \"receipt_footer\": null, \"receipt_header\": null}, \"returns\": [], \"payments\": [{\"amount\": \"2.00\", \"source\": \"pos\", \"status\": \"posted\", \"reference\": null, \"created_at\": \"2026-10-18T06:08:10.386968Z\", \"payment_id\": \"01a14d9f-f292-766e-a402-b9f17b997f05\", \"payment_no\": \"RCV-2610-00001\", \"method_name\": \"Cash\", \"account_name\": \"Cash\"}], \"subtotal\": \"2.00\", \"due_total\": \"0.00\", \"sale_date\": \"2026-10-18T06:08:10.376860Z\", \"tax_total\": \"0.10\", \"cashier_id\": \"01a14c8f-f094-75ea-92a8-5fc5af151c9e\", \"cost_total\": \"0.00\", \"created_at\": \"2026-10-18T06:08:10.382556Z\", \"invoice_no\": \"INV-2610-00001\", \"is_walk_in\": true, \"item_count\": 1, \"paid_total\": \"2.00\", \"customer_id\": \"11f1cab9-efc2-e801-ab00-bc542fed17aa\", \"grand_total\": \"2.00\", \"cancelled_at\": null, \"cashier_name\": \"Test Owner\", \"change_given\": \"0.00\", \"other_charge\": \"0.00\", \"business_date\": \"2026-10-18\", \"cancel_reason\": null, \"customer_name\": \"Walk-in Customer\", \"reprint_count\": 0, \"customer_phone\": null, \"payment_status\": \"paid\", \"returned_total\": \"0.00\", \"shipping_charge\": \"0.00\", \"prices_include_tax\": false, \"item_discount_total\": \"0.00\", \"rounding_adjustment\": \"-0.10\", \"invoice_discount_type\": null, \"invoice_discount_value\": null, \"invoice_discount_amount\": \"0.00\"}', 'sale', 0x01a14d9ff28c702f800284db35995a81, '2026-10-18 06:08:10.343662', '2026-10-21 06:08:10.343662'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0xdec75e594c854cf4a7cf718cbe4d7783, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /sales', 0x0591d8fde293690335ff02c24322735c0877207bf295ba1f0bbe2680c894c9d0, 'completed', 201, '{\"id\": \"01a173cb-f16c-7776-8110-a202cf51cead\", \"note\": null, \"items\": [{\"id\": \"01a173cb-f16f-70b2-9051-2806280fab9d\", \"sku\": \"Ata-001\", \"line_no\": 1, \"quantity\": \"1.000\", \"unit_cost\": \"0.0000\", \"cost_total\": \"0.00\", \"line_total\": \"2.10\", \"list_price\": \"2.0000\", \"tax_amount\": \"0.10\", \"unit_price\": \"2.0000\", \"variant_id\": \"01a14c91-a5fa-7226-9dd2-4dd5a08717cf\", \"gross_amount\": \"2.00\", \"product_name\": \"Ata 800\", \"tax_rate_pct\": \"5.0000\", \"discount_type\": null, \"line_discount\": \"0.00\", \"discount_value\": null, \"taxable_amount\": \"2.00\", \"track_inventory\": false, \"unit_short_name\": \"kg\", \"price_overridden\": false, \"returned_quantity\": \"0.000\", \"invoice_discount_alloc\": \"0.00\"}], \"status\": \"completed\", \"receipt\": {\"phone\": \"+8801700000001\", \"address\": null, \"auto_print\": true, \"bin_number\": null, \"branch_name\": \"Main Branch\", \"business_name\": \"Test Shop\", \"receipt_paper\": \"80mm\", \"receipt_footer\": null, \"receipt_header\": null}, \"returns\": [], \"payments\": [{\"amount\": \"2.00\", \"source\": \"pos\", \"status\": \"posted\", \"reference\": null, \"created_at\": \"2026-10-25T16:01:47.894162Z\", \"payment_id\": \"01a173cb-f175-73d1-8ec2-5959c5d447e8\", \"payment_no\": \"RCV-2610-00008\", \"method_name\": \"Cash\", \"account_name\": \"Cash\"}], \"subtotal\": \"2.00\", \"due_total\": \"0.00\", \"sale_date\": \"2026-10-25T16:01:47.879688Z\", \"tax_total\": \"0.10\", \"cashier_id\": \"01a14c8f-f094-75ea-92a8-5fc5af151c9e\", \"cost_total\": \"0.00\", \"created_at\": \"2026-10-25T16:01:47.887823Z\", \"invoice_no\": \"INV-2610-00007\", \"is_walk_in\": true, \"item_count\": 1, \"paid_total\": \"2.00\", \"customer_id\": \"11f1cab9-efc2-e801-ab00-bc542fed17aa\", \"grand_total\": \"2.00\", \"cancelled_at\": null, \"cashier_name\": \"Test Owner\", \"change_given\": \"31.00\", \"other_charge\": \"0.00\", \"business_date\": \"2026-10-25\", \"cancel_reason\": null, \"customer_name\": \"Walk-in Customer\", \"reprint_count\": 0, \"customer_phone\": null, \"payment_status\": \"paid\", \"returned_total\": \"0.00\", \"shipping_charge\": \"0.00\", \"prices_include_tax\": false, \"item_discount_total\": \"0.00\", \"rounding_adjustment\": \"-0.10\", \"invoice_discount_type\": null, \"invoice_discount_value\": null, \"invoice_discount_amount\": \"0.00\"}', 'sale', 0x01a173cbf16c77768110a202cf51cead, '2026-10-25 16:01:47.878064', '2026-10-28 16:01:47.878064'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0xe8120fb33bd943629ec2bedf8d8db935, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /purchases/post', 0x0ca2d7138448005beb70a3355608a25d8cf3fdfe01225cd0139a4bf2152f1f1a, 'completed', 200, '{\"id\": \"01a173e2-9933-711b-bff0-746df7a0951b\", \"note\": null, \"items\": [{\"id\": \"01a173e2-99b3-70b5-93e7-634198c2b81c\", \"sku\": \"008\", \"line_no\": 1, \"quantity\": \"1.000\", \"unit_cost\": \"4.0000\", \"line_total\": \"4.00\", \"tax_amount\": \"0.00\", \"variant_id\": \"01a173e1-bc2f-72d3-8cbf-3f359dbef54a\", \"product_name\": \"Pen\", \"tax_rate_pct\": \"0.0000\", \"charges_alloc\": \"0.00\", \"discount_type\": null, \"line_discount\": \"0.00\", \"discount_value\": null, \"unit_short_name\": \"pcs\", \"landed_unit_cost\": \"4.0000\", \"received_quantity\": \"1.000\", \"returned_quantity\": \"0.000\", \"previous_unit_cost\": null, \"bill_discount_alloc\": \"0.00\"}], \"status\": \"received\", \"returns\": [], \"version\": 2, \"payments\": [], \"receipts\": [{\"id\": \"01a173e2-9c65-768c-b83f-c880e0178154\", \"note\": null, \"value\": \"4.00\", \"item_count\": 1, \"receipt_no\": \"GRN-2610-00001\", \"received_at\": \"2026-10-25T16:26:33.797902Z\", \"business_date\": \"2026-10-25\", \"location_name\": \"Shop Floor\", \"received_by_name\": \"Test Owner\"}], \"subtotal\": \"4.00\", \"closed_at\": null, \"due_total\": \"4.00\", \"posted_at\": \"2026-10-25T16:26:33.908503Z\", \"tax_total\": \"0.00\", \"created_at\": \"2026-10-25T16:26:32.631045Z\", \"item_count\": 1, \"ordered_at\": null, \"other_cost\": \"0.00\", \"paid_total\": \"0.00\", \"approved_at\": null, \"grand_total\": \"4.00\", \"location_id\": \"01a14c8f-f30c-75c6-b3be-f99b21380151\", \"purchase_no\": \"PUR-2610-00001\", \"supplier_id\": \"01a151d6-8d09-74f8-bbb2-203ca6b1e0e0\", \"cancelled_at\": null, \"cancel_reason\": null, \"expected_date\": null, \"location_name\": \"Shop Floor\", \"purchase_date\": \"2026-10-25\", \"shipping_cost\": \"0.00\", \"supplier_name\": \"Mr Kashem\", \"payment_status\": \"unpaid\", \"received_total\": \"4.00\", \"returned_total\": \"0.00\", \"created_by_name\": \"Test Owner\", \"approved_by_name\": null, \"item_discount_total\": \"0.00\", \"supplier_invoice_no\": null, \"bill_discount_amount\": \"0.00\"}', 'purchase', 0x01a173e29933711bbff0746df7a0951b, '2026-10-25 16:26:33.336921', '2026-10-28 16:26:33.336921'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0xfa9ebfe2dcd54626b559b028530d78c4, 0x01a14c8ff09475ea92a85fc5af151c9e, 'POST /incomes', 0x4d461528bdc487179a71d6fcb63ad0304131a1c9579f18b14aaa7b819dfb9994, 'completed', 201, '{\"id\": \"01a1734d-ee10-76a7-99b9-747b07b9c05e\", \"note\": \"1\", \"amount\": \"11.00\", \"source\": \"1\", \"status\": \"posted\", \"income_no\": \"INC-2610-00001\", \"reference\": null, \"account_id\": \"11f1cab9-ed51-475a-ab00-bc542fed17aa\", \"created_at\": \"2026-10-25T13:44:09.490268Z\", \"payment_id\": \"01a1734d-ee1c-75d3-9453-d6538f1ea7ed\", \"category_id\": \"11f1cbe9-178d-5249-ab00-bc542fed17aa\", \"void_reason\": null, \"account_name\": \"Cash\", \"business_date\": \"2026-10-25\", \"category_name\": \"Rent Received\", \"created_by_name\": \"Test Owner\"}', 'other_income', 0x01a1734dee1076a799b9747b07b9c05e, '2026-10-25 13:44:09.311877', '2026-10-28 13:44:09.311877');

--
-- Triggers `idempotency_keys`
--
DELIMITER $$
CREATE TRIGGER `trg_idempotency_keys_tg_del` BEFORE DELETE ON `idempotency_keys` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_idempotency_keys_tg_ins` BEFORE INSERT ON `idempotency_keys` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_idempotency_keys_tg_upd` BEFORE UPDATE ON `idempotency_keys` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `income_categories`
--

CREATE TABLE `income_categories` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `name` varchar(100) NOT NULL,
  `ledger_account_id` binary(16) DEFAULT NULL,
  `is_system` tinyint(1) NOT NULL DEFAULT '0',
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `income_categories`
--

INSERT INTO `income_categories` (`tenant_id`, `id`, `name`, `ledger_account_id`, `is_system`, `status`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cbe9178d51b8ab00bc542fed17aa, 'Interest', 0x11f1cab9fdb0a832ab00bc542fed17aa, 1, 'active', '2026-10-19 18:15:40.812126'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cbe9178d5249ab00bc542fed17aa, 'Rent Received', 0x11f1cab9fdb0a832ab00bc542fed17aa, 1, 'active', '2026-10-19 18:15:40.812126'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cbe9178d5286ab00bc542fed17aa, 'Commission', 0x11f1cab9fdb0a832ab00bc542fed17aa, 1, 'active', '2026-10-19 18:15:40.812126'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cbe9178d52b3ab00bc542fed17aa, 'Scrap Sales', 0x11f1cab9fdb0a832ab00bc542fed17aa, 1, 'active', '2026-10-19 18:15:40.812126'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cbe9178d52dfab00bc542fed17aa, 'Miscellaneous Income', 0x11f1cab9fdb0a832ab00bc542fed17aa, 1, 'active', '2026-10-19 18:15:40.812126');

--
-- Triggers `income_categories`
--
DELIMITER $$
CREATE TRIGGER `trg_income_categories_tg_del` BEFORE DELETE ON `income_categories` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_income_categories_tg_ins` BEFORE INSERT ON `income_categories` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_income_categories_tg_upd` BEFORE UPDATE ON `income_categories` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `invitations`
--

CREATE TABLE `invitations` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `destination` varchar(254) NOT NULL,
  `token_hash` binary(32) NOT NULL,
  `role_ids` json NOT NULL,
  `branch_ids` json DEFAULT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'pending',
  `expires_at` datetime(6) NOT NULL,
  `accepted_by` binary(16) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `created_by` binary(16) NOT NULL
) ;

--
-- Triggers `invitations`
--
DELIMITER $$
CREATE TRIGGER `trg_invitations_tg_del` BEFORE DELETE ON `invitations` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_invitations_tg_ins` BEFORE INSERT ON `invitations` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_invitations_tg_upd` BEFORE UPDATE ON `invitations` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `journal_entries`
--

CREATE TABLE `journal_entries` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) DEFAULT NULL,
  `entry_no` varchar(40) NOT NULL,
  `entry_date` date NOT NULL,
  `source_type` varchar(30) NOT NULL,
  `source_id` binary(16) DEFAULT NULL,
  `source_no` varchar(40) DEFAULT NULL,
  `memo` varchar(500) DEFAULT NULL,
  `total_debit` decimal(18,2) NOT NULL,
  `total_credit` decimal(18,2) NOT NULL,
  `status` varchar(10) NOT NULL DEFAULT 'posted',
  `reversal_of_id` binary(16) DEFAULT NULL,
  `created_by` binary(16) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `journal_entries`
--

INSERT INTO `journal_entries` (`tenant_id`, `id`, `branch_id`, `entry_no`, `entry_date`, `source_type`, `source_id`, `source_no`, `memo`, `total_debit`, `total_credit`, `status`, `reversal_of_id`, `created_by`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9ff29976e6901ef04049695fc8, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00001', '2026-10-18', 'sale', 0x01a14d9ff28c702f800284db35995a81, 'INV-2610-00001', 'Sale INV-2610-00001 — Walk-in Customer', '2.10', '2.10', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-18 06:08:10.394748'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14da336d875e09c6f8e7f458dc787, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00002', '2026-10-18', 'sale', 0x01a14da336ca723dac4fec5f7d9b1aeb, 'INV-2610-00002', 'Sale INV-2610-00002 — Walk-in Customer', '6.00', '6.00', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-18 06:11:44.472503'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d33c837341ad5afe03d7ad7ef7, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00003', '2026-10-19', 'sale', 0x01a151d33c73715180219c1cedd1bb46, 'INV-2610-00003', 'Sale INV-2610-00003 — Walk-in Customer', '4.20', '4.20', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:42:40.516472'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d68d11702b9f72e853d0cce6cc, NULL, 'JV-2610-00004', '2026-10-19', 'opening_balance', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 'S000001', 'Opening balance: Mr Kashem', '100.00', '100.00', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:46:17.746091'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d724eb719f885b4d02ced01612, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00005', '2026-10-19', 'payment', 0x01a151d724e47199b13de8df2c8f054e, 'PMT-2610-00001', 'Payment to Mr Kashem', '100.00', '100.00', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:46:56.620301'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d7f755729cb1b81a180c37c991, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00006', '2026-10-19', 'expense', 0x01a151d7f749779b8799b76ffb0c0d82, 'EXP-2610-00001', 'Marketing', '1.00', '1.00', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:47:50.486327'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1521a06b072a2922ac6fbf64e0838, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00007', '2026-10-19', 'sale_return', 0x01a1521a069874d2a5190fdfcd9f0412, 'RET-2610-00001', 'Return RET-2610-00001 on INV-2610-00003', '2.10', '2.10', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 02:59:59.793235'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15563769c739e991cad7742174007, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00008', '2026-10-20', 'transfer', 0x01a155637515710e90247d06ca9ef0ad, 'TRF-2610-00001', 'Transfer Cash → Mobile Wallet', '10.00', '10.00', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 18:19:04.222960'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9fd9f771f1bca4387382344116, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00009', '2026-10-21', 'sale', 0x01a15a9fd9e6747db53070ca2ec2e354, 'INV-2610-00004', 'Sale INV-2610-00004 — Walk-in Customer', '4.20', '4.20', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-20 18:43:07.896643'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734def2976f984fa8f7a0708fd5f, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00010', '2026-10-25', 'other_income', 0x01a1734dee1076a799b9747b07b9c05e, 'INC-2610-00001', 'Rent Received — 1', '11.00', '11.00', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 13:44:09.772858'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734e4073763a866b7705e238f1eb, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00011', '2026-10-25', 'transfer', 0x01a1734e406b71b8875364222b7b6a82, 'TRF-2610-00002', 'Transfer Cash → Bank Account', '1.00', '1.00', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 13:44:30.580479'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c3ca5d7067a4a7d302528c77d8, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00012', '2026-10-25', 'sale', 0x01a173c3c7a476a48a17e9cd82ddd428, 'INV-2610-00005', 'Sale INV-2610-00005 — Walk-in Customer', '2.10', '2.10', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 15:52:53.603232'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cb7cb874799751d165cd34465d, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00013', '2026-10-25', 'sale', 0x01a173cb7caa7227ac4c13cf122599af, 'INV-2610-00006', 'Sale INV-2610-00006 — Walk-in Customer', '2.10', '2.10', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:01:18.009680'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cbf17c75b7b86cb4026d7b5a43, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00014', '2026-10-25', 'sale', 0x01a173cbf16c77768110a202cf51cead, 'INV-2610-00007', 'Sale INV-2610-00007 — Walk-in Customer', '2.10', '2.10', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:01:47.901640'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d0cf21758997eb33f4a9d535eb, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00015', '2026-10-25', 'sale', 0x01a173d0cf157236a45375e8a5272e85, 'INV-2610-00008', 'Sale INV-2610-00008 — Walk-in Customer', '2.10', '2.10', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:07:06.785227'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e29e26751787c23ff7ea607b44, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00016', '2026-10-25', 'purchase', 0x01a173e29933711bbff0746df7a0951b, 'PUR-2610-00001', 'Purchase PUR-2610-00001 / GRN-2610-00001 — Mr Kashem', '4.00', '4.00', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:26:33.896702'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e2d3d476c3b19295e0516cdd9d, 0x01a14c8ff30876da9be77f7076393059, 'JV-2610-00017', '2026-10-25', 'payment', 0x01a173e2d3c375b891737cae376653cb, 'PMT-2610-00004', 'Payment to Mr Kashem', '4.00', '4.00', 'posted', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:26:47.637863');

--
-- Triggers `journal_entries`
--
DELIMITER $$
CREATE TRIGGER `trg_journal_entries_period` BEFORE INSERT ON `journal_entries` FOR EACH ROW BEGIN
  IF EXISTS (SELECT 1 FROM fiscal_periods
             WHERE tenant_id = NEW.tenant_id AND status = 'locked'
               AND NEW.entry_date BETWEEN start_date AND end_date) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'PERIOD_LOCKED';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_journal_entries_tg_del` BEFORE DELETE ON `journal_entries` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_journal_entries_tg_ins` BEFORE INSERT ON `journal_entries` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_journal_entries_tg_upd` BEFORE UPDATE ON `journal_entries` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `journal_lines`
--

CREATE TABLE `journal_lines` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `entry_id` binary(16) NOT NULL,
  `entry_date` date NOT NULL,
  `account_id` binary(16) NOT NULL,
  `debit` decimal(18,2) NOT NULL DEFAULT '0.00',
  `credit` decimal(18,2) NOT NULL DEFAULT '0.00',
  `party_type` varchar(10) DEFAULT NULL,
  `party_id` binary(16) DEFAULT NULL,
  `branch_id` binary(16) DEFAULT NULL,
  `memo` varchar(255) DEFAULT NULL
) ;

--
-- Dumping data for table `journal_lines`
--

INSERT INTO `journal_lines` (`tenant_id`, `id`, `entry_id`, `entry_date`, `account_id`, `debit`, `credit`, `party_type`, `party_id`, `branch_id`, `memo`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9ff29b71908cb8cd4c8d678902, 0x01a14d9ff29976e6901ef04049695fc8, '2026-10-18', 0x11f1cab9fdb06614ab00bc542fed17aa, '2.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9ff29b71908cb8cd4d8377ad54, 0x01a14d9ff29976e6901ef04049695fc8, '2026-10-18', 0x11f1cab9fdb083feab00bc542fed17aa, '0.00', '0.10', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9ff29b71908cb8cd4e41fc8b9b, 0x01a14d9ff29976e6901ef04049695fc8, '2026-10-18', 0x11f1cab9fdb09e8cab00bc542fed17aa, '0.00', '2.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9ff29b71908cb8cd4f3acf6900, 0x01a14d9ff29976e6901ef04049695fc8, '2026-10-18', 0x11f1cab9fdb0ab63ab00bc542fed17aa, '0.10', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14da336d875e09c6f8e809f160d65, 0x01a14da336d875e09c6f8e7f458dc787, '2026-10-18', 0x11f1cab9fdb06614ab00bc542fed17aa, '6.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14da336d972ba96923278a8eac821, 0x01a14da336d875e09c6f8e7f458dc787, '2026-10-18', 0x11f1cab9fdb083feab00bc542fed17aa, '0.00', '0.27', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14da336d972ba9692327996e34438, 0x01a14da336d875e09c6f8e7f458dc787, '2026-10-18', 0x11f1cab9fdb09e8cab00bc542fed17aa, '0.00', '5.40', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14da336d972ba9692327a8c6aabb0, 0x01a14da336d875e09c6f8e7f458dc787, '2026-10-18', 0x11f1cab9fdb0ab63ab00bc542fed17aa, '0.00', '0.33', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d33c8575cb870adaab9ad02ae0, 0x01a151d33c837341ad5afe03d7ad7ef7, '2026-10-19', 0x11f1cab9fdb06614ab00bc542fed17aa, '4.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d33c8575cb870adaacd246818f, 0x01a151d33c837341ad5afe03d7ad7ef7, '2026-10-19', 0x11f1cab9fdb083feab00bc542fed17aa, '0.00', '0.20', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d33c8575cb870adaadfc42d0cc, 0x01a151d33c837341ad5afe03d7ad7ef7, '2026-10-19', 0x11f1cab9fdb09e8cab00bc542fed17aa, '0.00', '4.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d33c8575cb870adaae985d4298, 0x01a151d33c837341ad5afe03d7ad7ef7, '2026-10-19', 0x11f1cab9fdb0ab63ab00bc542fed17aa, '0.20', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d68d1273d484abf8bb18afdf3a, 0x01a151d68d11702b9f72e853d0cce6cc, '2026-10-19', 0x11f1cab9fdb080c5ab00bc542fed17aa, '0.00', '100.00', 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, NULL, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d68d1273d484abf8bce903139b, 0x01a151d68d11702b9f72e853d0cce6cc, '2026-10-19', 0x11f1cab9fdb094c9ab00bc542fed17aa, '100.00', '0.00', NULL, NULL, NULL, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d724ec738d97d7c1abddc26345, 0x01a151d724eb719f885b4d02ced01612, '2026-10-19', 0x11f1cab9fdb06614ab00bc542fed17aa, '0.00', '100.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d724ec738d97d7c1acd4a1b077, 0x01a151d724eb719f885b4d02ced01612, '2026-10-19', 0x11f1cab9fdb080c5ab00bc542fed17aa, '100.00', '0.00', 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d7f756700f9bb1ab0e94d571bc, 0x01a151d7f755729cb1b81a180c37c991, '2026-10-19', 0x11f1cab9fdb06614ab00bc542fed17aa, '0.00', '1.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d7f756700f9bb1ab0f59540737, 0x01a151d7f755729cb1b81a180c37c991, '2026-10-19', 0x11f1cab9fdb0c6d1ab00bc542fed17aa, '1.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1521a06b171c2b5b7b4901daaf0d5, 0x01a1521a06b072a2922ac6fbf64e0838, '2026-10-19', 0x11f1cab9fdb06614ab00bc542fed17aa, '0.00', '2.10', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1521a06b171c2b5b7b491ada627e8, 0x01a1521a06b072a2922ac6fbf64e0838, '2026-10-19', 0x11f1cab9fdb083feab00bc542fed17aa, '0.10', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1521a06b171c2b5b7b492a9e8cc71, 0x01a1521a06b072a2922ac6fbf64e0838, '2026-10-19', 0x11f1cab9fdb0a1caab00bc542fed17aa, '2.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1556376a172b2aea18b68c183a3df, 0x01a15563769c739e991cad7742174007, '2026-10-20', 0x11f1cab9fdb06614ab00bc542fed17aa, '0.00', '10.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1556376a172b2aea18b69499f6de7, 0x01a15563769c739e991cad7742174007, '2026-10-20', 0x11f1cab9fdb06fbfab00bc542fed17aa, '10.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9fda507299ba66c59f6986668f, 0x01a15a9fd9f771f1bca4387382344116, '2026-10-21', 0x11f1cab9fdb06614ab00bc542fed17aa, '4.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9fda507299ba66c5a090e4ebc3, 0x01a15a9fd9f771f1bca4387382344116, '2026-10-21', 0x11f1cab9fdb083feab00bc542fed17aa, '0.00', '0.20', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9fda507299ba66c5a13680677c, 0x01a15a9fd9f771f1bca4387382344116, '2026-10-21', 0x11f1cab9fdb09e8cab00bc542fed17aa, '0.00', '4.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9fda507299ba66c5a249370ebb, 0x01a15a9fd9f771f1bca4387382344116, '2026-10-21', 0x11f1cab9fdb0ab63ab00bc542fed17aa, '0.20', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734def2f7510b209ced1da6b3ff9, 0x01a1734def2976f984fa8f7a0708fd5f, '2026-10-25', 0x11f1cab9fdb06614ab00bc542fed17aa, '11.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734def2f7510b209ced2968288b7, 0x01a1734def2976f984fa8f7a0708fd5f, '2026-10-25', 0x11f1cab9fdb0a832ab00bc542fed17aa, '0.00', '11.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734e407477bea8d656f867228c84, 0x01a1734e4073763a866b7705e238f1eb, '2026-10-25', 0x11f1cab9fdb06614ab00bc542fed17aa, '0.00', '1.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734e407477bea8d656f958131c7b, 0x01a1734e4073763a866b7705e238f1eb, '2026-10-25', 0x11f1cab9fdb06be0ab00bc542fed17aa, '1.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c3ca7a771a8cca2fe910d03e23, 0x01a173c3ca5d7067a4a7d302528c77d8, '2026-10-25', 0x11f1cab9fdb06614ab00bc542fed17aa, '2.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c3ca7a771a8cca2fea863fa3e3, 0x01a173c3ca5d7067a4a7d302528c77d8, '2026-10-25', 0x11f1cab9fdb083feab00bc542fed17aa, '0.00', '0.10', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c3ca7a771a8cca2febdf18c03b, 0x01a173c3ca5d7067a4a7d302528c77d8, '2026-10-25', 0x11f1cab9fdb09e8cab00bc542fed17aa, '0.00', '2.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c3ca7a771a8cca2fec3b0f443e, 0x01a173c3ca5d7067a4a7d302528c77d8, '2026-10-25', 0x11f1cab9fdb0ab63ab00bc542fed17aa, '0.10', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cb7cba70bb9036345d36f639ce, 0x01a173cb7cb874799751d165cd34465d, '2026-10-25', 0x11f1cab9fdb06614ab00bc542fed17aa, '2.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cb7cba70bb9036345e62d42e44, 0x01a173cb7cb874799751d165cd34465d, '2026-10-25', 0x11f1cab9fdb083feab00bc542fed17aa, '0.00', '0.10', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cb7cba70bb9036345f54526451, 0x01a173cb7cb874799751d165cd34465d, '2026-10-25', 0x11f1cab9fdb09e8cab00bc542fed17aa, '0.00', '2.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cb7cba70bb90363460eac41f1c, 0x01a173cb7cb874799751d165cd34465d, '2026-10-25', 0x11f1cab9fdb0ab63ab00bc542fed17aa, '0.10', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cbf17e7301b8fb13dacd8046a5, 0x01a173cbf17c75b7b86cb4026d7b5a43, '2026-10-25', 0x11f1cab9fdb06614ab00bc542fed17aa, '2.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cbf17e7301b8fb13db3e67a825, 0x01a173cbf17c75b7b86cb4026d7b5a43, '2026-10-25', 0x11f1cab9fdb083feab00bc542fed17aa, '0.00', '0.10', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cbf17e7301b8fb13dcf4288a51, 0x01a173cbf17c75b7b86cb4026d7b5a43, '2026-10-25', 0x11f1cab9fdb09e8cab00bc542fed17aa, '0.00', '2.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cbf17e7301b8fb13ddd16c86f8, 0x01a173cbf17c75b7b86cb4026d7b5a43, '2026-10-25', 0x11f1cab9fdb0ab63ab00bc542fed17aa, '0.10', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d0cf21758997eb33f55218d267, 0x01a173d0cf21758997eb33f4a9d535eb, '2026-10-25', 0x11f1cab9fdb06614ab00bc542fed17aa, '2.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d0cf21758997eb33f640002013, 0x01a173d0cf21758997eb33f4a9d535eb, '2026-10-25', 0x11f1cab9fdb083feab00bc542fed17aa, '0.00', '0.10', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d0cf21758997eb33f742573172, 0x01a173d0cf21758997eb33f4a9d535eb, '2026-10-25', 0x11f1cab9fdb09e8cab00bc542fed17aa, '0.00', '2.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d0cf21758997eb33f80c54812b, 0x01a173d0cf21758997eb33f4a9d535eb, '2026-10-25', 0x11f1cab9fdb0ab63ab00bc542fed17aa, '0.10', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e29e2a7151a535e30807fc975e, 0x01a173e29e26751787c23ff7ea607b44, '2026-10-25', 0x11f1cab9fdb076c7ab00bc542fed17aa, '4.00', '0.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e29e2b73fe91e0c9bfab7ab32b, 0x01a173e29e26751787c23ff7ea607b44, '2026-10-25', 0x11f1cab9fdb080c5ab00bc542fed17aa, '0.00', '4.00', 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e2d3d675f5859d26cd679c2ef9, 0x01a173e2d3d476c3b19295e0516cdd9d, '2026-10-25', 0x11f1cab9fdb06614ab00bc542fed17aa, '0.00', '4.00', NULL, NULL, 0x01a14c8ff30876da9be77f7076393059, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e2d3d675f5859d26ce59aeb8e8, 0x01a173e2d3d476c3b19295e0516cdd9d, '2026-10-25', 0x11f1cab9fdb080c5ab00bc542fed17aa, '4.00', '0.00', 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 0x01a14c8ff30876da9be77f7076393059, NULL);

--
-- Triggers `journal_lines`
--
DELIMITER $$
CREATE TRIGGER `trg_journal_lines_ao_del` BEFORE DELETE ON `journal_lines` FOR EACH ROW BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_journal_lines_ao_upd` BEFORE UPDATE ON `journal_lines` FOR EACH ROW BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_journal_lines_tg_ins` BEFORE INSERT ON `journal_lines` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `ledger_accounts`
--

CREATE TABLE `ledger_accounts` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `code` varchar(20) NOT NULL,
  `name` varchar(150) NOT NULL,
  `account_type` varchar(10) NOT NULL,
  `normal_side` varchar(6) NOT NULL,
  `parent_id` binary(16) DEFAULT NULL,
  `system_key` varchar(40) DEFAULT NULL,
  `is_postable` tinyint(1) NOT NULL DEFAULT '1',
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `ledger_accounts`
--

INSERT INTO `ledger_accounts` (`tenant_id`, `id`, `code`, `name`, `account_type`, `normal_side`, `parent_id`, `system_key`, `is_postable`, `status`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb059b7ab00bc542fed17aa, '1000', 'Assets', 'asset', 'debit', NULL, NULL, 0, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb06614ab00bc542fed17aa, '1010', 'Cash in Hand', 'asset', 'debit', 0x11f1cab9fdb059b7ab00bc542fed17aa, 'cash', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb06be0ab00bc542fed17aa, '1020', 'Bank Accounts', 'asset', 'debit', 0x11f1cab9fdb059b7ab00bc542fed17aa, 'bank', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb06fbfab00bc542fed17aa, '1030', 'Mobile Wallets', 'asset', 'debit', 0x11f1cab9fdb059b7ab00bc542fed17aa, 'mobile_wallet', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0734dab00bc542fed17aa, '1100', 'Accounts Receivable', 'asset', 'debit', 0x11f1cab9fdb059b7ab00bc542fed17aa, 'ar', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb076c7ab00bc542fed17aa, '1200', 'Inventory', 'asset', 'debit', 0x11f1cab9fdb059b7ab00bc542fed17aa, 'inventory', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb07a11ab00bc542fed17aa, '1300', 'VAT Receivable (Input)', 'asset', 'debit', 0x11f1cab9fdb059b7ab00bc542fed17aa, 'vat_receivable', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb07d4bab00bc542fed17aa, '2000', 'Liabilities', 'liability', 'credit', NULL, NULL, 0, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb080c5ab00bc542fed17aa, '2100', 'Accounts Payable', 'liability', 'credit', 0x11f1cab9fdb07d4bab00bc542fed17aa, 'ap', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb083feab00bc542fed17aa, '2200', 'VAT Payable (Output)', 'liability', 'credit', 0x11f1cab9fdb07d4bab00bc542fed17aa, 'vat_payable', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb08866ab00bc542fed17aa, '2300', 'Customer Advances', 'liability', 'credit', 0x11f1cab9fdb07d4bab00bc542fed17aa, 'customer_advances', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb08bf9ab00bc542fed17aa, '3000', 'Equity', 'equity', 'credit', NULL, NULL, 0, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb09136ab00bc542fed17aa, '3100', 'Owner\'s Capital', 'equity', 'credit', 0x11f1cab9fdb08bf9ab00bc542fed17aa, 'owner_capital', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb094c9ab00bc542fed17aa, '3200', 'Opening Balance Equity', 'equity', 'credit', 0x11f1cab9fdb08bf9ab00bc542fed17aa, 'opening_equity', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0980cab00bc542fed17aa, '3300', 'Retained Earnings', 'equity', 'credit', 0x11f1cab9fdb08bf9ab00bc542fed17aa, 'retained_earnings', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb09b4bab00bc542fed17aa, '4000', 'Income', 'income', 'credit', NULL, NULL, 0, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb09e8cab00bc542fed17aa, '4100', 'Sales', 'income', 'credit', 0x11f1cab9fdb09b4bab00bc542fed17aa, 'sales', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0a1caab00bc542fed17aa, '4110', 'Sales Returns', 'income', 'debit', 0x11f1cab9fdb09b4bab00bc542fed17aa, 'sales_returns', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0a4feab00bc542fed17aa, '4120', 'Sales Discounts', 'income', 'debit', 0x11f1cab9fdb09b4bab00bc542fed17aa, 'sales_discount', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0a832ab00bc542fed17aa, '4200', 'Other Income', 'income', 'credit', 0x11f1cab9fdb09b4bab00bc542fed17aa, 'other_income', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0ab63ab00bc542fed17aa, '4300', 'Rounding Gain/Loss', 'income', 'credit', 0x11f1cab9fdb09b4bab00bc542fed17aa, 'rounding', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0affaab00bc542fed17aa, '5000', 'Cost of Goods Sold', 'expense', 'debit', NULL, 'cogs', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0b382ab00bc542fed17aa, '5100', 'Inventory Adjustments', 'expense', 'debit', NULL, 'inventory_adjustment', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0b6c7ab00bc542fed17aa, '6000', 'Operating Expenses', 'expense', 'debit', NULL, NULL, 0, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0ba14ab00bc542fed17aa, '6010', 'Rent', 'expense', 'debit', 0x11f1cab9fdb0b6c7ab00bc542fed17aa, NULL, 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0bd53ab00bc542fed17aa, '6020', 'Utilities', 'expense', 'debit', 0x11f1cab9fdb0b6c7ab00bc542fed17aa, NULL, 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0c072ab00bc542fed17aa, '6030', 'Salaries', 'expense', 'debit', 0x11f1cab9fdb0b6c7ab00bc542fed17aa, NULL, 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0c39aab00bc542fed17aa, '6040', 'Transport', 'expense', 'debit', 0x11f1cab9fdb0b6c7ab00bc542fed17aa, NULL, 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0c6d1ab00bc542fed17aa, '6050', 'Marketing', 'expense', 'debit', 0x11f1cab9fdb0b6c7ab00bc542fed17aa, NULL, 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0c9e2ab00bc542fed17aa, '6060', 'Maintenance', 'expense', 'debit', 0x11f1cab9fdb0b6c7ab00bc542fed17aa, NULL, 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0cce7ab00bc542fed17aa, '6070', 'Bank & MFS Charges', 'expense', 'debit', 0x11f1cab9fdb0b6c7ab00bc542fed17aa, 'bank_charges', 1, 'active', '2026-10-18 06:05:59.909330'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9fdb0d01fab00bc542fed17aa, '6080', 'Miscellaneous', 'expense', 'debit', 0x11f1cab9fdb0b6c7ab00bc542fed17aa, 'misc_expense', 1, 'active', '2026-10-18 06:05:59.909330');

--
-- Triggers `ledger_accounts`
--
DELIMITER $$
CREATE TRIGGER `trg_ledger_accounts_tg_del` BEFORE DELETE ON `ledger_accounts` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_ledger_accounts_tg_ins` BEFORE INSERT ON `ledger_accounts` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_ledger_accounts_tg_upd` BEFORE UPDATE ON `ledger_accounts` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `membership_roles`
--

CREATE TABLE `membership_roles` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `membership_id` binary(16) NOT NULL,
  `role_id` binary(16) NOT NULL,
  `branch_id` binary(16) DEFAULT NULL,
  `branch_key` binary(16) GENERATED ALWAYS AS (coalesce(`branch_id`,0x00000000000000000000000000000000)) VIRTUAL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Dumping data for table `membership_roles`
--

INSERT INTO `membership_roles` (`tenant_id`, `id`, `membership_id`, `role_id`, `branch_id`, `created_at`, `created_by`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff39a7709b70c6cff56372fce, 0x01a14c8ff39674da9a7c90acc3f1b94b, 0x01a14c8ff31372bdb9fa4e690b37bce0, NULL, '2026-10-18 01:11:04.860427', 0x01a14c8ff09475ea92a85fc5af151c9e);

--
-- Triggers `membership_roles`
--
DELIMITER $$
CREATE TRIGGER `trg_membership_roles_tg_del` BEFORE DELETE ON `membership_roles` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_membership_roles_tg_ins` BEFORE INSERT ON `membership_roles` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_membership_roles_tg_upd` BEFORE UPDATE ON `membership_roles` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `notifications`
--

CREATE TABLE `notifications` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `user_id` binary(16) NOT NULL,
  `notif_type` varchar(50) NOT NULL,
  `title` varchar(200) NOT NULL,
  `body` varchar(1000) DEFAULT NULL,
  `link` varchar(300) DEFAULT NULL,
  `data` json NOT NULL DEFAULT (json_object()),
  `dedupe_key` varchar(150) DEFAULT NULL,
  `read_at` datetime(6) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Triggers `notifications`
--
DELIMITER $$
CREATE TRIGGER `trg_notifications_tg_del` BEFORE DELETE ON `notifications` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_notifications_tg_ins` BEFORE INSERT ON `notifications` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_notifications_tg_upd` BEFORE UPDATE ON `notifications` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `notification_deliveries`
--

CREATE TABLE `notification_deliveries` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `notification_id` binary(16) NOT NULL,
  `channel` varchar(10) NOT NULL,
  `destination` varchar(254) NOT NULL,
  `status` varchar(10) NOT NULL DEFAULT 'queued',
  `attempts` smallint NOT NULL DEFAULT '0',
  `provider_ref` varchar(100) DEFAULT NULL,
  `last_error` varchar(1000) DEFAULT NULL,
  `next_attempt_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `sent_at` datetime(6) DEFAULT NULL
) ;

--
-- Triggers `notification_deliveries`
--
DELIMITER $$
CREATE TRIGGER `trg_notification_deliveries_tg_del` BEFORE DELETE ON `notification_deliveries` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_notification_deliveries_tg_ins` BEFORE INSERT ON `notification_deliveries` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_notification_deliveries_tg_upd` BEFORE UPDATE ON `notification_deliveries` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `notification_preferences`
--

CREATE TABLE `notification_preferences` (
  `tenant_id` binary(16) NOT NULL,
  `user_id` binary(16) NOT NULL,
  `notif_type` varchar(50) NOT NULL,
  `channels` json NOT NULL DEFAULT (json_array(_utf8mb4'in_app')),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Triggers `notification_preferences`
--
DELIMITER $$
CREATE TRIGGER `trg_notification_preferences_tg_del` BEFORE DELETE ON `notification_preferences` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_notification_preferences_tg_ins` BEFORE INSERT ON `notification_preferences` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_notification_preferences_tg_upd` BEFORE UPDATE ON `notification_preferences` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `other_incomes`
--

CREATE TABLE `other_incomes` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `income_no` varchar(40) NOT NULL,
  `category_id` binary(16) NOT NULL,
  `business_date` date NOT NULL,
  `amount` decimal(18,2) NOT NULL,
  `account_id` binary(16) NOT NULL,
  `payment_id` binary(16) DEFAULT NULL,
  `source` varchar(150) DEFAULT NULL,
  `reference` varchar(100) DEFAULT NULL,
  `note` varchar(500) DEFAULT NULL,
  `status` varchar(10) NOT NULL DEFAULT 'posted',
  `voided_at` datetime(6) DEFAULT NULL,
  `voided_by` binary(16) DEFAULT NULL,
  `void_reason` varchar(500) DEFAULT NULL,
  `created_by` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `other_incomes`
--

INSERT INTO `other_incomes` (`tenant_id`, `id`, `branch_id`, `income_no`, `category_id`, `business_date`, `amount`, `account_id`, `payment_id`, `source`, `reference`, `note`, `status`, `voided_at`, `voided_by`, `void_reason`, `created_by`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734dee1076a799b9747b07b9c05e, 0x01a14c8ff30876da9be77f7076393059, 'INC-2610-00001', 0x11f1cbe9178d5249ab00bc542fed17aa, '2026-10-25', '11.00', 0x11f1cab9ed51475aab00bc542fed17aa, 0x01a1734dee1c75d39453d6538f1ea7ed, '1', NULL, '1', 'posted', NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 13:44:09.490268');

--
-- Triggers `other_incomes`
--
DELIMITER $$
CREATE TRIGGER `trg_other_incomes_tg_del` BEFORE DELETE ON `other_incomes` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_other_incomes_tg_ins` BEFORE INSERT ON `other_incomes` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_other_incomes_tg_upd` BEFORE UPDATE ON `other_incomes` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `otp_codes`
--

CREATE TABLE `otp_codes` (
  `id` binary(16) NOT NULL,
  `user_id` binary(16) DEFAULT NULL,
  `destination` varchar(254) NOT NULL,
  `purpose` varchar(20) NOT NULL,
  `code_hash` binary(32) NOT NULL,
  `attempts` smallint NOT NULL DEFAULT '0',
  `expires_at` datetime(6) NOT NULL,
  `consumed_at` datetime(6) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `otp_codes`
--

INSERT INTO `otp_codes` (`id`, `user_id`, `destination`, `purpose`, `code_hash`, `attempts`, `expires_at`, `consumed_at`, `created_at`) VALUES
(0x01a14c8ff09875458d06e668f0f38128, 0x01a14c8ff09475ea92a85fc5af151c9e, '+8801700000001', 'verify_phone', 0x377425095fc12ed519975d7fd9c32b17931dc0497d6349ad594067ae2b05aa58, 0, '2026-10-18 01:16:04.089336', '2026-10-18 01:11:04.292769', '2026-10-18 01:11:04.089336');

-- --------------------------------------------------------

--
-- Table structure for table `outbox_events`
--

CREATE TABLE `outbox_events` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `event_type` varchar(60) NOT NULL,
  `aggregate_type` varchar(40) NOT NULL,
  `aggregate_id` binary(16) NOT NULL,
  `payload` json NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `published_at` datetime(6) DEFAULT NULL,
  `attempts` int NOT NULL DEFAULT '0',
  `last_error` varchar(1000) DEFAULT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Triggers `outbox_events`
--
DELIMITER $$
CREATE TRIGGER `trg_outbox_events_tg_del` BEFORE DELETE ON `outbox_events` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_outbox_events_tg_ins` BEFORE INSERT ON `outbox_events` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_outbox_events_tg_upd` BEFORE UPDATE ON `outbox_events` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `party_ledger_entries`
--

CREATE TABLE `party_ledger_entries` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `party_type` varchar(10) NOT NULL,
  `party_id` binary(16) NOT NULL,
  `branch_id` binary(16) DEFAULT NULL,
  `entry_type` varchar(20) NOT NULL,
  `amount` decimal(18,2) NOT NULL,
  `balance_after` decimal(18,2) NOT NULL,
  `business_date` date NOT NULL,
  `reference_type` varchar(30) NOT NULL,
  `reference_id` binary(16) NOT NULL,
  `reference_no` varchar(40) DEFAULT NULL,
  `note` varchar(500) DEFAULT NULL,
  `created_by` binary(16) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `party_ledger_entries`
--

INSERT INTO `party_ledger_entries` (`tenant_id`, `id`, `party_type`, `party_id`, `branch_id`, `entry_type`, `amount`, `balance_after`, `business_date`, `reference_type`, `reference_id`, `reference_no`, `note`, `created_by`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d68d0c777fb0173b2e1f10a386, 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, NULL, 'opening', '100.00', '100.00', '2026-10-19', 'opening_balance', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 'S000001', 'Opening balance', 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:46:17.741989'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d724e9778690b7928ac45c6713, 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 0x01a14c8ff30876da9be77f7076393059, 'payment_out', '-100.00', '0.00', '2026-10-19', 'payment', 0x01a151d724e47199b13de8df2c8f054e, 'PMT-2610-00001', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:46:56.617770'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e29dd3765d8f8e911464687b20, 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 0x01a14c8ff30876da9be77f7076393059, 'purchase', '4.00', '4.00', '2026-10-25', 'purchase', 0x01a173e29933711bbff0746df7a0951b, 'PUR-2610-00001', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:26:33.814448'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e2d3d074a8a08fa9ee920faad4, 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 0x01a14c8ff30876da9be77f7076393059, 'payment_out', '-4.00', '0.00', '2026-10-25', 'payment', 0x01a173e2d3c375b891737cae376653cb, 'PMT-2610-00004', NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:26:47.633411');

--
-- Triggers `party_ledger_entries`
--
DELIMITER $$
CREATE TRIGGER `trg_party_ledger_entries_ao_del` BEFORE DELETE ON `party_ledger_entries` FOR EACH ROW BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_party_ledger_entries_ao_upd` BEFORE UPDATE ON `party_ledger_entries` FOR EACH ROW BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_party_ledger_entries_tg_ins` BEFORE INSERT ON `party_ledger_entries` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `payments`
--

CREATE TABLE `payments` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `payment_no` varchar(40) NOT NULL,
  `direction` varchar(3) NOT NULL,
  `payment_date` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `business_date` date NOT NULL,
  `party_type` varchar(10) DEFAULT NULL,
  `party_id` binary(16) DEFAULT NULL,
  `account_id` binary(16) NOT NULL,
  `payment_method_id` binary(16) NOT NULL,
  `amount` decimal(18,2) NOT NULL,
  `unallocated_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `reference` varchar(100) DEFAULT NULL,
  `note` varchar(500) DEFAULT NULL,
  `source` varchar(20) NOT NULL,
  `status` varchar(10) NOT NULL DEFAULT 'posted',
  `voided_at` datetime(6) DEFAULT NULL,
  `voided_by` binary(16) DEFAULT NULL,
  `void_reason` varchar(500) DEFAULT NULL,
  `register_shift_id` binary(16) DEFAULT NULL,
  `created_by` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `payments`
--

INSERT INTO `payments` (`tenant_id`, `id`, `branch_id`, `payment_no`, `direction`, `payment_date`, `business_date`, `party_type`, `party_id`, `account_id`, `payment_method_id`, `amount`, `unallocated_amount`, `reference`, `note`, `source`, `status`, `voided_at`, `voided_by`, `void_reason`, `register_shift_id`, `created_by`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9ff292766ea402b9f17b997f05, 0x01a14c8ff30876da9be77f7076393059, 'RCV-2610-00001', 'in', '2026-10-18 06:08:10.386968', '2026-10-18', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '2.00', '0.00', NULL, 'Sale INV-2610-00001', 'pos', 'posted', NULL, NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-18 06:08:10.386968'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14da336d171c196e49427b8d3097a, 0x01a14c8ff30876da9be77f7076393059, 'RCV-2610-00002', 'in', '2026-10-18 06:11:44.465837', '2026-10-18', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '6.00', '0.00', NULL, 'Sale INV-2610-00002', 'pos', 'posted', NULL, NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-18 06:11:44.465837'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d33c7c7573ba30980ef714912f, 0x01a14c8ff30876da9be77f7076393059, 'RCV-2610-00003', 'in', '2026-10-19 01:42:40.508845', '2026-10-19', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '4.00', '0.00', NULL, 'Sale INV-2610-00003', 'pos', 'posted', NULL, NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:42:40.508845'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d724e47199b13de8df2c8f054e, 0x01a14c8ff30876da9be77f7076393059, 'PMT-2610-00001', 'out', '2026-10-19 01:46:56.613207', '2026-10-19', 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '100.00', '100.00', '5', 'Payment to Mr Kashem', 'supplier_payment', 'posted', NULL, NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:46:56.613207'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d7f74d7268837546ff0cb78b63, 0x01a14c8ff30876da9be77f7076393059, 'PMT-2610-00002', 'out', '2026-10-19 01:47:50.477771', '2026-10-19', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '1.00', '0.00', NULL, 'Marketing', 'expense', 'posted', NULL, NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 01:47:50.477771'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1521a06a5741281ff7a324443dec9, 0x01a14c8ff30876da9be77f7076393059, 'PMT-2610-00003', 'out', '2026-10-19 02:59:59.782215', '2026-10-19', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '2.10', '0.00', 'RET-2610-00001', 'Refund RET-2610-00001 for INV-2610-00003', 'refund', 'posted', NULL, NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 02:59:59.782215'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9fd9ef7012946c3bc56619a222, 0x01a14c8ff30876da9be77f7076393059, 'RCV-2610-00004', 'in', '2026-10-20 18:43:07.888205', '2026-10-21', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '4.00', '0.00', NULL, 'Sale INV-2610-00004', 'pos', 'posted', NULL, NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-20 18:43:07.888205'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734dee1c75d39453d6538f1ea7ed, 0x01a14c8ff30876da9be77f7076393059, 'RCV-2610-00005', 'in', '2026-10-25 13:44:09.506062', '2026-10-25', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '11.00', '0.00', NULL, 'Rent Received — 1', 'income', 'posted', NULL, NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 13:44:09.506062'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c3c83b77c6987217e24cc6448b, 0x01a14c8ff30876da9be77f7076393059, 'RCV-2610-00006', 'in', '2026-10-25 15:52:53.054563', '2026-10-25', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '2.00', '0.00', NULL, 'Sale INV-2610-00005', 'pos', 'posted', NULL, NULL, NULL, 0x01a17386a31c75dfb6c82efa53e9ff79, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 15:52:53.054563'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cb7cb270779f68efc75313eb1b, 0x01a14c8ff30876da9be77f7076393059, 'RCV-2610-00007', 'in', '2026-10-25 16:01:18.002917', '2026-10-25', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '2.00', '0.00', NULL, 'Sale INV-2610-00006', 'pos', 'posted', NULL, NULL, NULL, 0x01a17386a31c75dfb6c82efa53e9ff79, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:01:18.002917'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cbf17573d18ec25959c5d447e8, 0x01a14c8ff30876da9be77f7076393059, 'RCV-2610-00008', 'in', '2026-10-25 16:01:47.894162', '2026-10-25', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '2.00', '0.00', NULL, 'Sale INV-2610-00007', 'pos', 'posted', NULL, NULL, NULL, 0x01a17386a31c75dfb6c82efa53e9ff79, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:01:47.894162'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d0cf1b75a0a5d16d55080ed558, 0x01a14c8ff30876da9be77f7076393059, 'RCV-2610-00009', 'in', '2026-10-25 16:07:06.779935', '2026-10-25', NULL, NULL, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '2.00', '0.00', NULL, 'Sale INV-2610-00008', 'pos', 'posted', NULL, NULL, NULL, 0x01a17386a31c75dfb6c82efa53e9ff79, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:07:06.779935'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e2d3c375b891737cae376653cb, 0x01a14c8ff30876da9be77f7076393059, 'PMT-2610-00004', 'out', '2026-10-25 16:26:47.620177', '2026-10-25', 'supplier', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 0x11f1cab9ed51475aab00bc542fed17aa, 0x11f1cab9ed5d0607ab00bc542fed17aa, '4.00', '0.00', NULL, 'Payment to Mr Kashem', 'supplier_payment', 'posted', NULL, NULL, NULL, 0x01a17386a31c75dfb6c82efa53e9ff79, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:26:47.620177');

--
-- Triggers `payments`
--
DELIMITER $$
CREATE TRIGGER `trg_payments_tg_del` BEFORE DELETE ON `payments` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_payments_tg_ins` BEFORE INSERT ON `payments` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_payments_tg_upd` BEFORE UPDATE ON `payments` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `payment_accounts`
--

CREATE TABLE `payment_accounts` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) DEFAULT NULL,
  `account_type` varchar(20) NOT NULL,
  `name` varchar(100) NOT NULL,
  `account_number` varchar(50) DEFAULT NULL,
  `bank_name` varchar(100) DEFAULT NULL,
  `opening_balance` decimal(18,2) NOT NULL DEFAULT '0.00',
  `current_balance` decimal(18,2) NOT NULL DEFAULT '0.00',
  `ledger_account_id` binary(16) DEFAULT NULL,
  `is_default` tinyint(1) NOT NULL DEFAULT '0',
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `payment_accounts`
--

INSERT INTO `payment_accounts` (`tenant_id`, `id`, `branch_id`, `account_type`, `name`, `account_number`, `bank_name`, `opening_balance`, `current_balance`, `ledger_account_id`, `is_default`, `status`, `version`, `created_at`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9ed51475aab00bc542fed17aa, NULL, 'cash', 'Cash', NULL, NULL, '0.00', '-83.10', 0x11f1cab9fdb06614ab00bc542fed17aa, 1, 'active', 1, '2026-10-18 06:05:32.367862', '2026-10-25 16:26:47.624362'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9ed5155c9ab00bc542fed17aa, NULL, 'bank', 'Bank Account', NULL, NULL, '0.00', '1.00', 0x11f1cab9fdb06be0ab00bc542fed17aa, 0, 'active', 1, '2026-10-18 06:05:32.367862', '2026-10-25 13:44:30.575503'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9ed515b95ab00bc542fed17aa, NULL, 'mobile_wallet', 'Mobile Wallet', NULL, NULL, '0.00', '10.00', 0x11f1cab9fdb06fbfab00bc542fed17aa, 0, 'active', 1, '2026-10-18 06:05:32.367862', '2026-10-19 18:19:04.169183');

--
-- Triggers `payment_accounts`
--
DELIMITER $$
CREATE TRIGGER `trg_payment_accounts_tg_del` BEFORE DELETE ON `payment_accounts` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_payment_accounts_tg_ins` BEFORE INSERT ON `payment_accounts` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_payment_accounts_tg_upd` BEFORE UPDATE ON `payment_accounts` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `payment_allocations`
--

CREATE TABLE `payment_allocations` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `payment_id` binary(16) NOT NULL,
  `document_type` varchar(20) NOT NULL,
  `document_id` binary(16) NOT NULL,
  `amount` decimal(18,2) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `reversed_at` datetime(6) DEFAULT NULL
) ;

--
-- Dumping data for table `payment_allocations`
--

INSERT INTO `payment_allocations` (`tenant_id`, `id`, `payment_id`, `document_type`, `document_id`, `amount`, `created_at`, `reversed_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9ff29372be91cceda69aafb40a, 0x01a14d9ff292766ea402b9f17b997f05, 'sale', 0x01a14d9ff28c702f800284db35995a81, '2.00', '2026-10-18 06:08:10.388525', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14da336d2749897e1a09f249c4971, 0x01a14da336d171c196e49427b8d3097a, 'sale', 0x01a14da336ca723dac4fec5f7d9b1aeb, '6.00', '2026-10-18 06:11:44.467048', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d33c7d72a4ab47dd4db5516dba, 0x01a151d33c7c7573ba30980ef714912f, 'sale', 0x01a151d33c73715180219c1cedd1bb46, '4.00', '2026-10-19 01:42:40.509919', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d7f74e743f92a044b483b40d84, 0x01a151d7f74d7268837546ff0cb78b63, 'expense', 0x01a151d7f749779b8799b76ffb0c0d82, '1.00', '2026-10-19 01:47:50.478833', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1521a06a6752297ecc04d4c64af51, 0x01a1521a06a5741281ff7a324443dec9, 'sale_return', 0x01a1521a069874d2a5190fdfcd9f0412, '2.10', '2026-10-19 02:59:59.783839', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9fd9f070b9860afda4a9bbda65, 0x01a15a9fd9ef7012946c3bc56619a222, 'sale', 0x01a15a9fd9e6747db53070ca2ec2e354, '4.00', '2026-10-20 18:43:07.889729', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1734dee24775c8e8377cb890c3166, 0x01a1734dee1c75d39453d6538f1ea7ed, 'other_income', 0x01a1734dee1076a799b9747b07b9c05e, '11.00', '2026-10-25 13:44:09.512365', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c3c840729b8a0743ecac10519c, 0x01a173c3c83b77c6987217e24cc6448b, 'sale', 0x01a173c3c7a476a48a17e9cd82ddd428, '2.00', '2026-10-25 15:52:53.059842', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cb7cb3770b89af7f6b99cf0da9, 0x01a173cb7cb270779f68efc75313eb1b, 'sale', 0x01a173cb7caa7227ac4c13cf122599af, '2.00', '2026-10-25 16:01:18.004022', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cbf1767271964ef26f2a4c70a8, 0x01a173cbf17573d18ec25959c5d447e8, 'sale', 0x01a173cbf16c77768110a202cf51cead, '2.00', '2026-10-25 16:01:47.895329', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d0cf1c76f59a56ff5cb1f9c64d, 0x01a173d0cf1b75a0a5d16d55080ed558, 'sale', 0x01a173d0cf157236a45375e8a5272e85, '2.00', '2026-10-25 16:07:06.780797', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e2d3c570399bfc3e05ad3d33f0, 0x01a173e2d3c375b891737cae376653cb, 'purchase', 0x01a173e29933711bbff0746df7a0951b, '4.00', '2026-10-25 16:26:47.622115', NULL);

--
-- Triggers `payment_allocations`
--
DELIMITER $$
CREATE TRIGGER `trg_payment_allocations_tg_del` BEFORE DELETE ON `payment_allocations` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_payment_allocations_tg_ins` BEFORE INSERT ON `payment_allocations` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_payment_allocations_tg_upd` BEFORE UPDATE ON `payment_allocations` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `payment_methods`
--

CREATE TABLE `payment_methods` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `name` varchar(50) NOT NULL,
  `method_type` varchar(20) NOT NULL,
  `default_account_id` binary(16) DEFAULT NULL,
  `requires_reference` tinyint(1) NOT NULL DEFAULT '0',
  `sort_order` int NOT NULL DEFAULT '0',
  `is_active` tinyint(1) NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `payment_methods`
--

INSERT INTO `payment_methods` (`tenant_id`, `id`, `name`, `method_type`, `default_account_id`, `requires_reference`, `sort_order`, `is_active`, `created_at`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9ed5d0607ab00bc542fed17aa, 'Cash', 'cash', 0x11f1cab9ed51475aab00bc542fed17aa, 0, 1, 1, '2026-10-18 06:05:32.521321', '2026-10-18 06:05:32.521321'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9ed5d11c6ab00bc542fed17aa, 'Card', 'card', 0x11f1cab9ed5155c9ab00bc542fed17aa, 0, 2, 1, '2026-10-18 06:05:32.521321', '2026-10-18 06:05:32.521321'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9ed5d174aab00bc542fed17aa, 'bKash', 'mobile_wallet', 0x11f1cab9ed515b95ab00bc542fed17aa, 1, 3, 1, '2026-10-18 06:05:32.521321', '2026-10-18 06:05:32.521321'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9ed5d1b9aab00bc542fed17aa, 'Nagad', 'mobile_wallet', 0x11f1cab9ed515b95ab00bc542fed17aa, 1, 4, 1, '2026-10-18 06:05:32.521321', '2026-10-18 06:05:32.521321'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9ed5d1f7aab00bc542fed17aa, 'Rocket', 'mobile_wallet', 0x11f1cab9ed515b95ab00bc542fed17aa, 1, 5, 1, '2026-10-18 06:05:32.521321', '2026-10-18 06:05:32.521321'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x11f1cab9ed5d2353ab00bc542fed17aa, 'Bank Transfer', 'bank_transfer', 0x11f1cab9ed5155c9ab00bc542fed17aa, 1, 6, 1, '2026-10-18 06:05:32.521321', '2026-10-18 06:05:32.521321');

--
-- Triggers `payment_methods`
--
DELIMITER $$
CREATE TRIGGER `trg_payment_methods_tg_del` BEFORE DELETE ON `payment_methods` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_payment_methods_tg_ins` BEFORE INSERT ON `payment_methods` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_payment_methods_tg_upd` BEFORE UPDATE ON `payment_methods` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `permissions`
--

CREATE TABLE `permissions` (
  `code` varchar(60) NOT NULL,
  `module` varchar(30) NOT NULL,
  `description` varchar(255) NOT NULL,
  `is_sensitive` tinyint(1) NOT NULL DEFAULT '0'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Dumping data for table `permissions`
--

INSERT INTO `permissions` (`code`, `module`, `description`, `is_sensitive`) VALUES
('accounting.journal_create', 'accounting', 'Create manual journal entries', 1),
('accounting.period_lock', 'accounting', 'Lock and unlock fiscal periods', 1),
('accounting.view', 'accounting', 'View ledgers and statements', 0),
('accounts.manage', 'money', 'Manage cash/bank accounts', 1),
('accounts.view', 'money', 'View cash/bank accounts and books', 0),
('audit.view', 'audit', 'View audit log', 1),
('branches.manage', 'settings', 'Manage branches', 1),
('customers.collect_payment', 'customers', 'Collect customer payments', 0),
('customers.manage', 'customers', 'Create and update customers', 0),
('customers.view', 'customers', 'View customers', 0),
('dashboard.view', 'dashboard', 'View dashboard', 0),
('dashboard.view_financials', 'dashboard', 'View financial figures on dashboard', 1),
('expenses.approve', 'money', 'Approve expenses', 1),
('expenses.create', 'money', 'Record expenses', 0),
('expenses.view', 'money', 'View expenses', 0),
('income.create', 'money', 'Record other income', 0),
('inventory.adjust', 'inventory', 'Post stock adjustments', 1),
('inventory.count', 'inventory', 'Run stock counts', 0),
('inventory.opening_stock', 'inventory', 'Enter opening stock', 1),
('inventory.transfer', 'inventory', 'Create and dispatch stock transfers', 0),
('inventory.transfer_receive', 'inventory', 'Receive stock transfers', 0),
('inventory.view', 'inventory', 'View stock', 0),
('payments.void', 'money', 'Void posted payments', 1),
('pos.below_min_price', 'pos', 'Sell below minimum selling price', 1),
('pos.discount_over_limit', 'pos', 'Give discounts above the role limit', 1),
('pos.override_stock', 'pos', 'Sell when stock is insufficient', 1),
('pos.price_override', 'pos', 'Change the unit price in the POS', 1),
('pos.sell', 'pos', 'Use the POS and complete sales', 0),
('products.archive', 'products', 'Archive products', 0),
('products.create', 'products', 'Create products', 0),
('products.import', 'products', 'Import products from files', 0),
('products.update', 'products', 'Update products and prices', 0),
('products.view', 'products', 'View products', 0),
('products.view_cost', 'products', 'View purchase cost and margins', 1),
('purchases.approve', 'purchases', 'Approve purchase orders', 1),
('purchases.cancel', 'purchases', 'Cancel purchases', 1),
('purchases.create', 'purchases', 'Create purchase drafts and orders', 0),
('purchases.post', 'purchases', 'Post purchase bills', 0),
('purchases.return', 'purchases', 'Process purchase returns', 1),
('purchases.view', 'purchases', 'View purchases', 0),
('reports.export', 'reports', 'Export reports', 1),
('reports.financial', 'reports', 'View financial reports', 1),
('reports.inventory', 'reports', 'View inventory reports', 0),
('reports.parties', 'reports', 'View customer and supplier reports', 0),
('reports.purchases', 'reports', 'View purchase reports', 0),
('reports.sales', 'reports', 'View sales reports', 0),
('sales.backdate', 'sales', 'Record sales with a past date', 1),
('sales.cancel', 'sales', 'Cancel (void) a completed sale', 1),
('sales.override_credit_limit', 'sales', 'Exceed a customer credit limit', 1),
('sales.return', 'sales', 'Process sale returns', 1),
('sales.return_override', 'sales', 'Return outside the return window', 1),
('sales.view', 'sales', 'View own sales', 0),
('sales.view_all_users', 'sales', 'View sales of all users', 0),
('settings.manage', 'settings', 'Change business settings', 1),
('settings.view', 'settings', 'View business settings', 0),
('subscription.manage', 'settings', 'Manage subscription and billing', 1),
('suppliers.manage', 'suppliers', 'Create and update suppliers', 0),
('suppliers.pay', 'suppliers', 'Pay suppliers', 1),
('suppliers.view', 'suppliers', 'View suppliers', 0),
('transfers.create', 'money', 'Transfer money between accounts', 1),
('users.invite', 'users', 'Invite users', 1),
('users.manage_roles', 'users', 'Change roles and permissions', 1),
('users.revoke_sessions', 'users', 'Revoke user sessions', 1),
('users.view', 'users', 'View users', 0);

-- --------------------------------------------------------

--
-- Table structure for table `plans`
--

CREATE TABLE `plans` (
  `id` binary(16) NOT NULL,
  `code` varchar(40) NOT NULL,
  `name` varchar(100) NOT NULL,
  `description` text,
  `is_public` tinyint(1) NOT NULL DEFAULT '1',
  `is_trial` tinyint(1) NOT NULL DEFAULT '0',
  `trial_days` int DEFAULT NULL,
  `sort_order` int NOT NULL DEFAULT '0',
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `plans`
--

INSERT INTO `plans` (`id`, `code`, `name`, `description`, `is_public`, `is_trial`, `trial_days`, `sort_order`, `status`, `created_at`, `updated_at`) VALUES
(0x019a0000000070008000000000000001, 'trial', 'Free Trial', NULL, 0, 1, 14, 0, 'active', '2026-10-18 01:05:38.451918', '2026-10-18 01:05:38.451918'),
(0x019a0000000070008000000000000002, 'starter', 'Starter', NULL, 1, 0, NULL, 1, 'active', '2026-10-18 01:05:38.451918', '2026-10-18 01:05:38.451918'),
(0x019a0000000070008000000000000003, 'business', 'Business', NULL, 1, 0, NULL, 2, 'active', '2026-10-18 01:05:38.451918', '2026-10-18 01:05:38.451918'),
(0x019a0000000070008000000000000004, 'enterprise', 'Enterprise', NULL, 0, 0, NULL, 3, 'active', '2026-10-18 01:05:38.451918', '2026-10-18 01:05:38.451918');

-- --------------------------------------------------------

--
-- Table structure for table `plan_features`
--

CREATE TABLE `plan_features` (
  `plan_id` binary(16) NOT NULL,
  `feature_code` varchar(60) NOT NULL,
  `enabled` tinyint(1) NOT NULL DEFAULT '1',
  `limit_value` bigint DEFAULT NULL
) ;

--
-- Dumping data for table `plan_features`
--

INSERT INTO `plan_features` (`plan_id`, `feature_code`, `enabled`, `limit_value`) VALUES
(0x019a0000000070008000000000000001, 'accounting.double_entry', 1, NULL),
(0x019a0000000070008000000000000001, 'api.access', 0, NULL),
(0x019a0000000070008000000000000001, 'branches.max', 1, 1),
(0x019a0000000070008000000000000001, 'inventory.adjustments', 1, NULL),
(0x019a0000000070008000000000000001, 'inventory.transfers', 0, NULL),
(0x019a0000000070008000000000000001, 'invoices.monthly_max', 1, 1000),
(0x019a0000000070008000000000000001, 'notifications.sms', 0, NULL),
(0x019a0000000070008000000000000001, 'pos.core', 1, NULL),
(0x019a0000000070008000000000000001, 'products.max', 1, 500),
(0x019a0000000070008000000000000001, 'products.variants', 1, NULL),
(0x019a0000000070008000000000000001, 'purchases.orders', 1, NULL),
(0x019a0000000070008000000000000001, 'registers.max', 1, 1),
(0x019a0000000070008000000000000001, 'reports.advanced', 1, NULL),
(0x019a0000000070008000000000000001, 'reports.export', 1, NULL),
(0x019a0000000070008000000000000001, 'storage.mb', 1, 200),
(0x019a0000000070008000000000000001, 'users.max', 1, 3),
(0x019a0000000070008000000000000001, 'whitelabel', 0, NULL),
(0x019a0000000070008000000000000002, 'accounting.double_entry', 0, NULL),
(0x019a0000000070008000000000000002, 'api.access', 0, NULL),
(0x019a0000000070008000000000000002, 'branches.max', 1, 1),
(0x019a0000000070008000000000000002, 'inventory.adjustments', 1, NULL),
(0x019a0000000070008000000000000002, 'inventory.transfers', 0, NULL),
(0x019a0000000070008000000000000002, 'invoices.monthly_max', 1, 5000),
(0x019a0000000070008000000000000002, 'notifications.sms', 0, NULL),
(0x019a0000000070008000000000000002, 'pos.core', 1, NULL),
(0x019a0000000070008000000000000002, 'products.max', 1, 2000),
(0x019a0000000070008000000000000002, 'products.variants', 1, NULL),
(0x019a0000000070008000000000000002, 'purchases.orders', 0, NULL),
(0x019a0000000070008000000000000002, 'registers.max', 1, 1),
(0x019a0000000070008000000000000002, 'reports.advanced', 0, NULL),
(0x019a0000000070008000000000000002, 'reports.export', 1, NULL),
(0x019a0000000070008000000000000002, 'storage.mb', 1, 500),
(0x019a0000000070008000000000000002, 'users.max', 1, 3),
(0x019a0000000070008000000000000002, 'whitelabel', 0, NULL),
(0x019a0000000070008000000000000003, 'accounting.double_entry', 1, NULL),
(0x019a0000000070008000000000000003, 'api.access', 0, NULL),
(0x019a0000000070008000000000000003, 'branches.max', 1, 3),
(0x019a0000000070008000000000000003, 'inventory.adjustments', 1, NULL),
(0x019a0000000070008000000000000003, 'inventory.transfers', 1, NULL),
(0x019a0000000070008000000000000003, 'invoices.monthly_max', 1, NULL),
(0x019a0000000070008000000000000003, 'notifications.sms', 1, NULL),
(0x019a0000000070008000000000000003, 'pos.core', 1, NULL),
(0x019a0000000070008000000000000003, 'products.max', 1, NULL),
(0x019a0000000070008000000000000003, 'products.variants', 1, NULL),
(0x019a0000000070008000000000000003, 'purchases.orders', 1, NULL),
(0x019a0000000070008000000000000003, 'registers.max', 1, 5),
(0x019a0000000070008000000000000003, 'reports.advanced', 1, NULL),
(0x019a0000000070008000000000000003, 'reports.export', 1, NULL),
(0x019a0000000070008000000000000003, 'storage.mb', 1, 5000),
(0x019a0000000070008000000000000003, 'users.max', 1, 15),
(0x019a0000000070008000000000000003, 'whitelabel', 0, NULL),
(0x019a0000000070008000000000000004, 'accounting.double_entry', 1, NULL),
(0x019a0000000070008000000000000004, 'api.access', 1, NULL),
(0x019a0000000070008000000000000004, 'branches.max', 1, NULL),
(0x019a0000000070008000000000000004, 'inventory.adjustments', 1, NULL),
(0x019a0000000070008000000000000004, 'inventory.transfers', 1, NULL),
(0x019a0000000070008000000000000004, 'invoices.monthly_max', 1, NULL),
(0x019a0000000070008000000000000004, 'notifications.sms', 1, NULL),
(0x019a0000000070008000000000000004, 'pos.core', 1, NULL),
(0x019a0000000070008000000000000004, 'products.max', 1, NULL),
(0x019a0000000070008000000000000004, 'products.variants', 1, NULL),
(0x019a0000000070008000000000000004, 'purchases.orders', 1, NULL),
(0x019a0000000070008000000000000004, 'registers.max', 1, NULL),
(0x019a0000000070008000000000000004, 'reports.advanced', 1, NULL),
(0x019a0000000070008000000000000004, 'reports.export', 1, NULL),
(0x019a0000000070008000000000000004, 'storage.mb', 1, NULL),
(0x019a0000000070008000000000000004, 'users.max', 1, NULL),
(0x019a0000000070008000000000000004, 'whitelabel', 1, NULL);

-- --------------------------------------------------------

--
-- Table structure for table `plan_prices`
--

CREATE TABLE `plan_prices` (
  `id` binary(16) NOT NULL,
  `plan_id` binary(16) NOT NULL,
  `billing_cycle` varchar(20) NOT NULL,
  `currency_code` char(3) NOT NULL DEFAULT 'BDT',
  `amount` decimal(18,2) NOT NULL,
  `is_active` tinyint(1) NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `plan_prices`
--

INSERT INTO `plan_prices` (`id`, `plan_id`, `billing_cycle`, `currency_code`, `amount`, `is_active`, `created_at`) VALUES
(0x019a0000000070008001000000000001, 0x019a0000000070008000000000000002, 'monthly', 'BDT', '990.00', 1, '2026-10-18 01:05:38.497246'),
(0x019a0000000070008001000000000002, 0x019a0000000070008000000000000002, 'quarterly', 'BDT', '2821.50', 1, '2026-10-18 01:05:38.497246'),
(0x019a0000000070008001000000000003, 0x019a0000000070008000000000000002, 'annual', 'BDT', '10098.00', 1, '2026-10-18 01:05:38.497246'),
(0x019a0000000070008001000000000004, 0x019a0000000070008000000000000003, 'monthly', 'BDT', '2490.00', 1, '2026-10-18 01:05:38.497246'),
(0x019a0000000070008001000000000005, 0x019a0000000070008000000000000003, 'quarterly', 'BDT', '7096.50', 1, '2026-10-18 01:05:38.497246'),
(0x019a0000000070008001000000000006, 0x019a0000000070008000000000000003, 'annual', 'BDT', '25398.00', 1, '2026-10-18 01:05:38.497246');

-- --------------------------------------------------------

--
-- Table structure for table `platform_sequences`
--

CREATE TABLE `platform_sequences` (
  `name` varchar(50) NOT NULL,
  `next_value` bigint NOT NULL DEFAULT '1'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Dumping data for table `platform_sequences`
--

INSERT INTO `platform_sequences` (`name`, `next_value`) VALUES
('subscription_invoice', 3);

-- --------------------------------------------------------

--
-- Table structure for table `platform_settings`
--

CREATE TABLE `platform_settings` (
  `setting_key` varchar(100) NOT NULL,
  `value` json NOT NULL,
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `updated_by` binary(16) DEFAULT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Dumping data for table `platform_settings`
--

INSERT INTO `platform_settings` (`setting_key`, `value`, `updated_at`, `updated_by`) VALUES
('billing.payment_instructions', '{\"note\": \"Your subscription is extended as soon as the payment is verified (usually within a few hours).\", \"bkash\": \"Send Money to 01XXXXXXXXX (bKash personal) and enter the TrxID.\", \"nagad\": \"Send Money to 01XXXXXXXXX (Nagad) and enter the transaction ID.\", \"bank_transfer\": \"Deposit to account 0000000000, Bank Name, Branch; enter the deposit reference.\"}', '2026-10-19 02:07:34.778939', NULL);

-- --------------------------------------------------------

--
-- Table structure for table `platform_staff`
--

CREATE TABLE `platform_staff` (
  `user_id` binary(16) NOT NULL,
  `staff_role` varchar(20) NOT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `mfa_required` tinyint(1) NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL
) ;

--
-- Dumping data for table `platform_staff`
--

INSERT INTO `platform_staff` (`user_id`, `staff_role`, `status`, `mfa_required`, `created_at`, `created_by`) VALUES
(0x01a14c8ff09475ea92a85fc5af151c9e, 'platform_owner', 'active', 1, '2026-10-19 02:07:35.801199', NULL);

-- --------------------------------------------------------

--
-- Table structure for table `products`
--

CREATE TABLE `products` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `product_type` varchar(20) NOT NULL DEFAULT 'standard',
  `name` varchar(200) NOT NULL,
  `name_bn` varchar(200) DEFAULT NULL,
  `search_name` varchar(400) NOT NULL,
  `description` text,
  `category_id` binary(16) DEFAULT NULL,
  `brand_id` binary(16) DEFAULT NULL,
  `base_unit_id` binary(16) NOT NULL,
  `tax_rate_id` binary(16) DEFAULT NULL,
  `track_inventory` tinyint(1) NOT NULL DEFAULT '1',
  `has_variants` tinyint(1) NOT NULL DEFAULT '0',
  `image_file_id` binary(16) DEFAULT NULL,
  `attributes` json NOT NULL DEFAULT (json_object()),
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `is_sample` tinyint(1) NOT NULL DEFAULT '0',
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL,
  `updated_by` binary(16) DEFAULT NULL,
  `deleted_at` datetime(6) DEFAULT NULL
) ;

--
-- Dumping data for table `products`
--

INSERT INTO `products` (`tenant_id`, `id`, `product_type`, `name`, `name_bn`, `search_name`, `description`, `category_id`, `brand_id`, `base_unit_id`, `tax_rate_id`, `track_inventory`, `has_variants`, `image_file_id`, `attributes`, `status`, `is_sample`, `version`, `created_at`, `updated_at`, `created_by`, `updated_by`, `deleted_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c91a5fa72269dd24dd4ee7deede, 'standard', 'Ata 800', 'অাটা', 'Ata 800 অাটা', NULL, 0x01a173da393271caa334171028125df8, 0x01a173da52d976e4837a27669b51c389, 0x01a14c8ff2f9761eb9f638875ad3ce54, 0x01a14c8ff2ed71d6a7c4621fce01d7eb, 0, 0, NULL, '{}', 'active', 0, 3, '2026-10-18 01:12:56.059144', '2026-10-25 16:18:09.619048', 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e1bc2f72d38cbf3f345a7e5305, 'standard', 'Pen', 'কলম', 'Pen কলম', NULL, 0x01a173da393271caa334171028125df8, 0x01a173da52d976e4837a27669b51c389, 0x01a14c8ff2f677f693a223fc85d990ab, 0x01a14c8ff2e9708e988ff9bfb894aaf2, 1, 0, NULL, '{}', 'active', 0, 1, '2026-10-25 16:25:36.049331', '2026-10-25 16:25:36.049331', 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL);

--
-- Triggers `products`
--
DELIMITER $$
CREATE TRIGGER `trg_products_tg_del` BEFORE DELETE ON `products` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_products_tg_ins` BEFORE INSERT ON `products` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_products_tg_upd` BEFORE UPDATE ON `products` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `product_barcodes`
--

CREATE TABLE `product_barcodes` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `barcode` varchar(64) NOT NULL,
  `symbology` varchar(10) DEFAULT NULL,
  `unit_id` binary(16) DEFAULT NULL,
  `is_primary` tinyint(1) NOT NULL DEFAULT '0',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `product_barcodes`
--

INSERT INTO `product_barcodes` (`tenant_id`, `id`, `variant_id`, `barcode`, `symbology`, `unit_id`, `is_primary`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e1bcb07419ab1ead0d07cb0554, 0x01a173e1bc2f72d38cbf3f359dbef54a, '008', 'other', NULL, 1, '2026-10-25 16:25:36.177982');

--
-- Triggers `product_barcodes`
--
DELIMITER $$
CREATE TRIGGER `trg_product_barcodes_tg_del` BEFORE DELETE ON `product_barcodes` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_product_barcodes_tg_ins` BEFORE INSERT ON `product_barcodes` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_product_barcodes_tg_upd` BEFORE UPDATE ON `product_barcodes` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `product_price_history`
--

CREATE TABLE `product_price_history` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `price_field` varchar(20) NOT NULL,
  `old_value` decimal(18,4) DEFAULT NULL,
  `new_value` decimal(18,4) DEFAULT NULL,
  `changed_by` binary(16) DEFAULT NULL,
  `changed_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Triggers `product_price_history`
--
DELIMITER $$
CREATE TRIGGER `trg_product_price_history_tg_del` BEFORE DELETE ON `product_price_history` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_product_price_history_tg_ins` BEFORE INSERT ON `product_price_history` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_product_price_history_tg_upd` BEFORE UPDATE ON `product_price_history` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `product_variants`
--

CREATE TABLE `product_variants` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `product_id` binary(16) NOT NULL,
  `sku` varchar(64) NOT NULL,
  `variant_name` varchar(100) DEFAULT NULL,
  `option_values` json NOT NULL DEFAULT (json_object()),
  `is_default` tinyint(1) NOT NULL DEFAULT '0',
  `purchase_price` decimal(18,4) NOT NULL DEFAULT '0.0000',
  `selling_price` decimal(18,4) NOT NULL DEFAULT '0.0000',
  `wholesale_price` decimal(18,4) DEFAULT NULL,
  `min_selling_price` decimal(18,4) DEFAULT NULL,
  `reorder_level` decimal(18,3) DEFAULT NULL,
  `search_text` varchar(500) NOT NULL,
  `last_purchase_price` decimal(18,4) DEFAULT NULL,
  `last_purchase_date` date DEFAULT NULL,
  `last_purchase_qty` decimal(18,3) DEFAULT NULL,
  `last_supplier_id` binary(16) DEFAULT NULL,
  `last_purchase_item_id` binary(16) DEFAULT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `deleted_at` datetime(6) DEFAULT NULL,
  `live_sku` varchar(64) GENERATED ALWAYS AS (if((`deleted_at` is null),`sku`,NULL)) VIRTUAL,
  `default_key` tinyint GENERATED ALWAYS AS (if(`is_default`,1,NULL)) VIRTUAL
) ;

--
-- Dumping data for table `product_variants`
--

INSERT INTO `product_variants` (`tenant_id`, `id`, `product_id`, `sku`, `variant_name`, `option_values`, `is_default`, `purchase_price`, `selling_price`, `wholesale_price`, `min_selling_price`, `reorder_level`, `search_text`, `last_purchase_price`, `last_purchase_date`, `last_purchase_qty`, `last_supplier_id`, `last_purchase_item_id`, `status`, `version`, `created_at`, `updated_at`, `deleted_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c91a5fa72269dd24dd5a08717cf, 0x01a14c91a5fa72269dd24dd4ee7deede, 'Ata-001', NULL, '{}', 1, '2.0000', '2.0000', '2.0000', '2.0000', NULL, 'Ata 800 অাটা Ata-001', NULL, NULL, NULL, NULL, NULL, 'active', 3, '2026-10-18 01:12:56.060518', '2026-10-25 16:18:09.620444', NULL),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e1bc2f72d38cbf3f359dbef54a, 0x01a173e1bc2f72d38cbf3f345a7e5305, '008', NULL, '{}', 1, '4.0000', '5.0000', '4.5000', '60.0000', '100.000', 'Pen কলম 008', '4.0000', '2026-10-25', '1.000', 0x01a151d68d0974f8bbb2203ca6b1e0e0, 0x01a173e299b370b593e7634198c2b81c, 'active', 1, '2026-10-25 16:25:36.128857', '2026-10-25 16:26:33.805599', NULL);

--
-- Triggers `product_variants`
--
DELIMITER $$
CREATE TRIGGER `trg_product_variants_tg_del` BEFORE DELETE ON `product_variants` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_product_variants_tg_ins` BEFORE INSERT ON `product_variants` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_product_variants_tg_upd` BEFORE UPDATE ON `product_variants` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `purchases`
--

CREATE TABLE `purchases` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `location_id` binary(16) NOT NULL,
  `purchase_no` varchar(40) DEFAULT NULL,
  `supplier_id` binary(16) NOT NULL,
  `supplier_invoice_no` varchar(60) DEFAULT NULL,
  `purchase_date` date NOT NULL,
  `expected_date` date DEFAULT NULL,
  `ordered_at` datetime(6) DEFAULT NULL,
  `ordered_by` binary(16) DEFAULT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'draft',
  `payment_status` varchar(10) NOT NULL DEFAULT 'unpaid',
  `subtotal` decimal(18,2) NOT NULL DEFAULT '0.00',
  `item_discount_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `bill_discount_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `tax_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `shipping_cost` decimal(18,2) NOT NULL DEFAULT '0.00',
  `other_cost` decimal(18,2) NOT NULL DEFAULT '0.00',
  `grand_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `received_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `paid_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `returned_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `due_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `note` text,
  `attachment_file_id` binary(16) DEFAULT NULL,
  `approved_by` binary(16) DEFAULT NULL,
  `approved_at` datetime(6) DEFAULT NULL,
  `posted_at` datetime(6) DEFAULT NULL,
  `posted_by` binary(16) DEFAULT NULL,
  `closed_at` datetime(6) DEFAULT NULL,
  `closed_by` binary(16) DEFAULT NULL,
  `cancelled_at` datetime(6) DEFAULT NULL,
  `cancelled_by` binary(16) DEFAULT NULL,
  `cancel_reason` varchar(500) DEFAULT NULL,
  `idempotency_key` binary(16) DEFAULT NULL,
  `version` int NOT NULL DEFAULT '1',
  `created_by` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `live_supplier_invoice` varchar(60) GENERATED ALWAYS AS (if((`status` <> _utf8mb4'cancelled'),`supplier_invoice_no`,NULL)) VIRTUAL
) ;

--
-- Dumping data for table `purchases`
--

INSERT INTO `purchases` (`tenant_id`, `id`, `branch_id`, `location_id`, `purchase_no`, `supplier_id`, `supplier_invoice_no`, `purchase_date`, `expected_date`, `ordered_at`, `ordered_by`, `status`, `payment_status`, `subtotal`, `item_discount_total`, `bill_discount_amount`, `tax_total`, `shipping_cost`, `other_cost`, `grand_total`, `received_total`, `paid_total`, `returned_total`, `due_total`, `note`, `attachment_file_id`, `approved_by`, `approved_at`, `posted_at`, `posted_by`, `closed_at`, `closed_by`, `cancelled_at`, `cancelled_by`, `cancel_reason`, `idempotency_key`, `version`, `created_by`, `created_at`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e29933711bbff0746df7a0951b, 0x01a14c8ff30876da9be77f7076393059, 0x01a14c8ff30c75c6b3bef99b21380151, 'PUR-2610-00001', 0x01a151d68d0974f8bbb2203ca6b1e0e0, NULL, '2026-10-25', NULL, NULL, NULL, 'received', 'paid', '4.00', '0.00', '0.00', '0.00', '0.00', '0.00', '4.00', '4.00', '4.00', '0.00', '0.00', NULL, NULL, NULL, NULL, '2026-10-25 16:26:33.908503', 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, NULL, NULL, NULL, NULL, 0xe8120fb33bd943629ec2bedf8d8db935, 2, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:26:32.631045', '2026-10-25 16:26:47.629403');

--
-- Triggers `purchases`
--
DELIMITER $$
CREATE TRIGGER `trg_purchases_tg_del` BEFORE DELETE ON `purchases` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_purchases_tg_ins` BEFORE INSERT ON `purchases` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_purchases_tg_upd` BEFORE UPDATE ON `purchases` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `purchase_items`
--

CREATE TABLE `purchase_items` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `purchase_id` binary(16) NOT NULL,
  `line_no` smallint NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `product_name` varchar(200) NOT NULL,
  `sku` varchar(64) NOT NULL,
  `unit_id` binary(16) NOT NULL,
  `unit_short_name` varchar(10) NOT NULL,
  `unit_factor` decimal(18,6) NOT NULL DEFAULT '1.000000',
  `quantity` decimal(18,3) NOT NULL,
  `base_quantity` decimal(18,3) NOT NULL,
  `received_quantity` decimal(18,3) NOT NULL DEFAULT '0.000',
  `returned_quantity` decimal(18,3) NOT NULL DEFAULT '0.000',
  `unit_cost` decimal(18,4) NOT NULL,
  `discount_type` varchar(10) DEFAULT NULL,
  `discount_value` decimal(18,4) DEFAULT NULL,
  `line_discount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `bill_discount_alloc` decimal(18,2) NOT NULL DEFAULT '0.00',
  `tax_rate_pct` decimal(7,4) NOT NULL DEFAULT '0.0000',
  `tax_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `charges_alloc` decimal(18,2) NOT NULL DEFAULT '0.00',
  `line_total` decimal(18,2) NOT NULL,
  `landed_unit_cost` decimal(18,4) NOT NULL,
  `previous_unit_cost` decimal(18,4) DEFAULT NULL
) ;

--
-- Dumping data for table `purchase_items`
--

INSERT INTO `purchase_items` (`tenant_id`, `id`, `purchase_id`, `line_no`, `variant_id`, `product_name`, `sku`, `unit_id`, `unit_short_name`, `unit_factor`, `quantity`, `base_quantity`, `received_quantity`, `returned_quantity`, `unit_cost`, `discount_type`, `discount_value`, `line_discount`, `bill_discount_alloc`, `tax_rate_pct`, `tax_amount`, `charges_alloc`, `line_total`, `landed_unit_cost`, `previous_unit_cost`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e299b370b593e7634198c2b81c, 0x01a173e29933711bbff0746df7a0951b, 1, 0x01a173e1bc2f72d38cbf3f359dbef54a, 'Pen', '008', 0x01a14c8ff2f677f693a223fc85d990ab, 'pcs', '1.000000', '1.000', '1.000', '1.000', '0.000', '4.0000', NULL, NULL, '0.00', '0.00', '0.0000', '0.00', '0.00', '4.00', '4.0000', NULL);

--
-- Triggers `purchase_items`
--
DELIMITER $$
CREATE TRIGGER `trg_purchase_items_tg_del` BEFORE DELETE ON `purchase_items` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_purchase_items_tg_ins` BEFORE INSERT ON `purchase_items` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_purchase_items_tg_upd` BEFORE UPDATE ON `purchase_items` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `purchase_returns`
--

CREATE TABLE `purchase_returns` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `return_no` varchar(40) NOT NULL,
  `purchase_id` binary(16) NOT NULL,
  `supplier_id` binary(16) NOT NULL,
  `location_id` binary(16) NOT NULL,
  `business_date` date NOT NULL,
  `reason` varchar(500) NOT NULL,
  `total` decimal(18,2) NOT NULL,
  `tax_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `cost_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `settlement_mode` varchar(20) NOT NULL,
  `applied_to_due` decimal(18,2) NOT NULL DEFAULT '0.00',
  `refunded_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `credited_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `idempotency_key` binary(16) NOT NULL,
  `created_by` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Triggers `purchase_returns`
--
DELIMITER $$
CREATE TRIGGER `trg_purchase_returns_tg_del` BEFORE DELETE ON `purchase_returns` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_purchase_returns_tg_ins` BEFORE INSERT ON `purchase_returns` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_purchase_returns_tg_upd` BEFORE UPDATE ON `purchase_returns` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `purchase_return_items`
--

CREATE TABLE `purchase_return_items` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `return_id` binary(16) NOT NULL,
  `purchase_item_id` binary(16) NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `base_quantity` decimal(18,3) NOT NULL,
  `tax_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `line_total` decimal(18,2) NOT NULL,
  `unit_cost_out` decimal(18,4) NOT NULL
) ;

--
-- Triggers `purchase_return_items`
--
DELIMITER $$
CREATE TRIGGER `trg_purchase_return_items_tg_del` BEFORE DELETE ON `purchase_return_items` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_purchase_return_items_tg_ins` BEFORE INSERT ON `purchase_return_items` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_purchase_return_items_tg_upd` BEFORE UPDATE ON `purchase_return_items` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `register_shifts`
--

CREATE TABLE `register_shifts` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `register_id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `shift_no` int NOT NULL,
  `opened_by` binary(16) NOT NULL,
  `opened_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `opening_cash` decimal(18,2) NOT NULL,
  `closed_by` binary(16) DEFAULT NULL,
  `closed_at` datetime(6) DEFAULT NULL,
  `expected_cash` decimal(18,2) DEFAULT NULL,
  `counted_cash` decimal(18,2) DEFAULT NULL,
  `difference` decimal(18,2) GENERATED ALWAYS AS ((`counted_cash` - `expected_cash`)) STORED,
  `status` varchar(10) NOT NULL DEFAULT 'open',
  `open_key` tinyint GENERATED ALWAYS AS (if((`status` = _utf8mb4'open'),1,NULL)) VIRTUAL,
  `acknowledged_by` binary(16) DEFAULT NULL,
  `note` varchar(500) DEFAULT NULL
) ;

--
-- Dumping data for table `register_shifts`
--

INSERT INTO `register_shifts` (`tenant_id`, `id`, `register_id`, `branch_id`, `shift_no`, `opened_by`, `opened_at`, `opening_cash`, `closed_by`, `closed_at`, `expected_cash`, `counted_cash`, `status`, `acknowledged_by`, `note`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a17386a31c75dfb6c82efa53e9ff79, 0x01a17386776f7022a112740b2ae9f46a, 0x01a14c8ff30876da9be77f7076393059, 1, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 14:46:05.853015', '11.00', NULL, NULL, NULL, NULL, 'open', NULL, NULL);

--
-- Triggers `register_shifts`
--
DELIMITER $$
CREATE TRIGGER `trg_register_shifts_tg_del` BEFORE DELETE ON `register_shifts` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_register_shifts_tg_ins` BEFORE INSERT ON `register_shifts` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_register_shifts_tg_upd` BEFORE UPDATE ON `register_shifts` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `roles`
--

CREATE TABLE `roles` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `code` varchar(40) NOT NULL,
  `name` varchar(100) NOT NULL,
  `description` varchar(255) DEFAULT NULL,
  `is_system` tinyint(1) NOT NULL DEFAULT '0',
  `max_discount_pct` decimal(7,4) DEFAULT NULL,
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `roles`
--

INSERT INTO `roles` (`tenant_id`, `id`, `code`, `name`, `description`, `is_system`, `max_discount_pct`, `version`, `created_at`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'owner', 'Owner', 'Full access, including subscription and ownership', 1, '100.0000', 1, '2026-10-18 01:11:04.726177', '2026-10-18 01:11:04.726177'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'admin', 'Admin', 'Full access except subscription; cannot remove the owner', 1, '100.0000', 1, '2026-10-18 01:11:04.739682', '2026-10-18 01:11:04.739682'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'branch_manager', 'Branch Manager', 'Runs a branch: sales, stock, purchases, parties, money and reports', 1, '25.0000', 1, '2026-10-18 01:11:04.755222', '2026-10-18 01:11:04.755222'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33c717f96a31e35108f81dc, 'cashier', 'Cashier', 'Sells at the POS, processes returns and collects customer payments', 1, NULL, 1, '2026-10-18 01:11:04.765380', '2026-10-18 01:11:04.765380'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34176268e2dbb81f855725c, 'salesperson', 'Salesperson', 'Sells at the POS and manages customers', 1, NULL, 1, '2026-10-18 01:11:04.769445', '2026-10-18 01:11:04.769445'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'store_manager', 'Store Manager', 'Manages the catalogue, stock and purchasing', 1, NULL, 1, '2026-10-18 01:11:04.774237', '2026-10-18 01:11:04.774237'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34d726b85e5d060e7ac25cd, 'warehouse', 'Warehouse', 'Receives, counts and moves stock', 1, NULL, 1, '2026-10-18 01:11:04.782290', '2026-10-18 01:11:04.782290'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'accountant', 'Accountant', 'Money, parties, accounting and financial reports', 1, NULL, 1, '2026-10-18 01:11:04.787173', '2026-10-18 01:11:04.787173');

--
-- Triggers `roles`
--
DELIMITER $$
CREATE TRIGGER `trg_roles_tg_del` BEFORE DELETE ON `roles` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_roles_tg_ins` BEFORE INSERT ON `roles` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_roles_tg_upd` BEFORE UPDATE ON `roles` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `role_permissions`
--

CREATE TABLE `role_permissions` (
  `tenant_id` binary(16) NOT NULL,
  `role_id` binary(16) NOT NULL,
  `permission_code` varchar(60) NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Dumping data for table `role_permissions`
--

INSERT INTO `role_permissions` (`tenant_id`, `role_id`, `permission_code`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'accounting.journal_create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'accounting.journal_create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'accounting.journal_create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'accounting.period_lock'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'accounting.period_lock'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'accounting.period_lock'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'accounting.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'accounting.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'accounting.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'accounts.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'accounts.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'accounts.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'accounts.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'accounts.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'accounts.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'accounts.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'accounts.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'audit.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'audit.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'branches.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'branches.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'customers.collect_payment'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'customers.collect_payment'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'customers.collect_payment'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33c717f96a31e35108f81dc, 'customers.collect_payment'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'customers.collect_payment'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'customers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'customers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'customers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33c717f96a31e35108f81dc, 'customers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34176268e2dbb81f855725c, 'customers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'customers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'customers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'customers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'customers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33c717f96a31e35108f81dc, 'customers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34176268e2dbb81f855725c, 'customers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'customers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'dashboard.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'dashboard.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'dashboard.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'dashboard.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'dashboard.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'dashboard.view_financials'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'dashboard.view_financials'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'dashboard.view_financials'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'dashboard.view_financials'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'expenses.approve'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'expenses.approve'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'expenses.approve'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'expenses.approve'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'expenses.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'expenses.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'expenses.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'expenses.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'expenses.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'expenses.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'expenses.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'expenses.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'income.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'income.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'income.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'income.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'inventory.adjust'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'inventory.adjust'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'inventory.adjust'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'inventory.adjust'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34d726b85e5d060e7ac25cd, 'inventory.adjust'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'inventory.count'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'inventory.count'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'inventory.count'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'inventory.count'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34d726b85e5d060e7ac25cd, 'inventory.count'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'inventory.opening_stock'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'inventory.opening_stock'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'inventory.opening_stock'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'inventory.transfer'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'inventory.transfer'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'inventory.transfer'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'inventory.transfer'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34d726b85e5d060e7ac25cd, 'inventory.transfer'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'inventory.transfer_receive'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'inventory.transfer_receive'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'inventory.transfer_receive'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'inventory.transfer_receive'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34d726b85e5d060e7ac25cd, 'inventory.transfer_receive'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'inventory.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'inventory.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'inventory.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33c717f96a31e35108f81dc, 'inventory.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34176268e2dbb81f855725c, 'inventory.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'inventory.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34d726b85e5d060e7ac25cd, 'inventory.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'inventory.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'payments.void'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'payments.void'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'payments.void'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'pos.below_min_price'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'pos.below_min_price'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'pos.discount_over_limit'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'pos.discount_over_limit'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'pos.discount_over_limit'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'pos.override_stock'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'pos.override_stock'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'pos.price_override'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'pos.price_override'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'pos.price_override'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'pos.sell'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'pos.sell'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'pos.sell'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33c717f96a31e35108f81dc, 'pos.sell'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34176268e2dbb81f855725c, 'pos.sell'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'products.archive'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'products.archive'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'products.archive'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'products.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'products.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'products.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'products.import'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'products.import'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'products.import'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'products.update'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'products.update'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'products.update'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'products.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'products.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'products.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33c717f96a31e35108f81dc, 'products.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34176268e2dbb81f855725c, 'products.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'products.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34d726b85e5d060e7ac25cd, 'products.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'products.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'products.view_cost'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'products.view_cost'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'products.view_cost'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'products.view_cost'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'products.view_cost'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'purchases.approve'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'purchases.approve'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'purchases.cancel'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'purchases.cancel'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'purchases.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'purchases.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'purchases.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'purchases.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34d726b85e5d060e7ac25cd, 'purchases.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'purchases.post'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'purchases.post'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'purchases.post'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'purchases.post'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'purchases.return'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'purchases.return'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'purchases.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'purchases.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'purchases.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'purchases.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34d726b85e5d060e7ac25cd, 'purchases.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'purchases.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'reports.export'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'reports.export'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'reports.export'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'reports.export'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'reports.export'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'reports.financial'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'reports.financial'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'reports.financial'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'reports.financial'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'reports.inventory'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'reports.inventory'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'reports.inventory'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'reports.inventory'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'reports.inventory'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'reports.parties'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'reports.parties'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'reports.parties'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'reports.parties'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'reports.purchases'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'reports.purchases'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'reports.purchases'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'reports.purchases'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'reports.sales'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'reports.sales'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'reports.sales'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'reports.sales'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'sales.backdate'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'sales.backdate'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'sales.cancel'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'sales.cancel'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'sales.cancel'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'sales.override_credit_limit'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'sales.override_credit_limit'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'sales.return'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'sales.return'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'sales.return'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33c717f96a31e35108f81dc, 'sales.return'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'sales.return_override'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'sales.return_override'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'sales.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'sales.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'sales.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33c717f96a31e35108f81dc, 'sales.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff34176268e2dbb81f855725c, 'sales.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'sales.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'sales.view_all_users'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'sales.view_all_users'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'sales.view_all_users'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'sales.view_all_users'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'settings.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'settings.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'settings.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'settings.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'subscription.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'suppliers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'suppliers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'suppliers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'suppliers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'suppliers.manage'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'suppliers.pay'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'suppliers.pay'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'suppliers.pay'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'suppliers.pay'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'suppliers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'suppliers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'suppliers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3457576943243817cf8e189, 'suppliers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'suppliers.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'transfers.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'transfers.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff33271fbb4b8d7a6c71291c5, 'transfers.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff35277a0b2a5b161e6a2cccf, 'transfers.create'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'users.invite'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'users.invite'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'users.manage_roles'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'users.manage_roles'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'users.revoke_sessions'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'users.revoke_sessions'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff31372bdb9fa4e690b37bce0, 'users.view'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff32376f7800324353e1c2c0c, 'users.view');

--
-- Triggers `role_permissions`
--
DELIMITER $$
CREATE TRIGGER `trg_role_permissions_tg_del` BEFORE DELETE ON `role_permissions` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_role_permissions_tg_ins` BEFORE INSERT ON `role_permissions` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_role_permissions_tg_upd` BEFORE UPDATE ON `role_permissions` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `sales`
--

CREATE TABLE `sales` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `location_id` binary(16) NOT NULL,
  `invoice_no` varchar(40) NOT NULL,
  `sale_date` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `business_date` date NOT NULL,
  `customer_id` binary(16) NOT NULL,
  `cashier_id` binary(16) NOT NULL,
  `salesperson_id` binary(16) DEFAULT NULL,
  `register_shift_id` binary(16) DEFAULT NULL,
  `price_list_id` binary(16) DEFAULT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'completed',
  `payment_status` varchar(10) NOT NULL,
  `prices_include_tax` tinyint(1) NOT NULL,
  `subtotal` decimal(18,2) NOT NULL,
  `item_discount_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `invoice_discount_type` varchar(10) DEFAULT NULL,
  `invoice_discount_value` decimal(18,4) DEFAULT NULL,
  `invoice_discount_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `tax_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `shipping_charge` decimal(18,2) NOT NULL DEFAULT '0.00',
  `other_charge` decimal(18,2) NOT NULL DEFAULT '0.00',
  `rounding_adjustment` decimal(18,2) NOT NULL DEFAULT '0.00',
  `grand_total` decimal(18,2) NOT NULL,
  `paid_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `returned_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `due_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `change_given` decimal(18,2) NOT NULL DEFAULT '0.00',
  `cost_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `note` varchar(500) DEFAULT NULL,
  `reprint_count` int NOT NULL DEFAULT '0',
  `idempotency_key` binary(16) NOT NULL,
  `cancelled_at` datetime(6) DEFAULT NULL,
  `cancelled_by` binary(16) DEFAULT NULL,
  `cancel_reason` varchar(500) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `sales`
--

INSERT INTO `sales` (`tenant_id`, `id`, `branch_id`, `location_id`, `invoice_no`, `sale_date`, `business_date`, `customer_id`, `cashier_id`, `salesperson_id`, `register_shift_id`, `price_list_id`, `status`, `payment_status`, `prices_include_tax`, `subtotal`, `item_discount_total`, `invoice_discount_type`, `invoice_discount_value`, `invoice_discount_amount`, `tax_total`, `shipping_charge`, `other_charge`, `rounding_adjustment`, `grand_total`, `paid_total`, `returned_total`, `due_total`, `change_given`, `cost_total`, `note`, `reprint_count`, `idempotency_key`, `cancelled_at`, `cancelled_by`, `cancel_reason`, `created_at`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9ff28c702f800284db35995a81, 0x01a14c8ff30876da9be77f7076393059, 0x01a14c8ff30c75c6b3bef99b21380151, 'INV-2610-00001', '2026-10-18 06:08:10.376860', '2026-10-18', 0x11f1cab9efc2e801ab00bc542fed17aa, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, NULL, NULL, 'completed', 'paid', 0, '2.00', '0.00', NULL, NULL, '0.00', '0.10', '0.00', '0.00', '-0.10', '2.00', '2.00', '0.00', '0.00', '0.00', '0.00', NULL, 0, 0xca51c7a57ada403e9321cc7bdbab9567, NULL, NULL, NULL, '2026-10-18 06:08:10.382556', '2026-10-18 06:08:10.382556'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14da336ca723dac4fec5f7d9b1aeb, 0x01a14c8ff30876da9be77f7076393059, 0x01a14c8ff30c75c6b3bef99b21380151, 'INV-2610-00002', '2026-10-18 06:11:44.455039', '2026-10-18', 0x11f1cab9efc2e801ab00bc542fed17aa, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, NULL, NULL, 'completed', 'paid', 0, '6.00', '0.60', NULL, NULL, '0.00', '0.27', '0.00', '0.00', '0.33', '6.00', '6.00', '0.00', '0.00', '0.00', '0.00', NULL, 0, 0x0a042dc0a80244408d9a762844addb67, NULL, NULL, NULL, '2026-10-18 06:11:44.461257', '2026-10-18 06:11:44.461257'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d33c73715180219c1cedd1bb46, 0x01a14c8ff30876da9be77f7076393059, 0x01a14c8ff30c75c6b3bef99b21380151, 'INV-2610-00003', '2026-10-19 01:42:40.494213', '2026-10-19', 0x11f1cab9efc2e801ab00bc542fed17aa, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, NULL, NULL, 'partially_returned', 'paid', 0, '4.00', '0.00', NULL, NULL, '0.00', '0.20', '0.00', '0.00', '-0.20', '4.00', '4.00', '2.10', '0.00', '0.00', '0.00', NULL, 0, 0x13ef1df8c7884a69a24734d0b4c10d04, NULL, NULL, NULL, '2026-10-19 01:42:40.503106', '2026-10-19 02:59:59.789131'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9fd9e6747db53070ca2ec2e354, 0x01a14c8ff30876da9be77f7076393059, 0x01a14c8ff30c75c6b3bef99b21380151, 'INV-2610-00004', '2026-10-20 18:43:07.871152', '2026-10-21', 0x11f1cab9efc2e801ab00bc542fed17aa, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, NULL, NULL, 'completed', 'paid', 0, '4.00', '0.00', NULL, NULL, '0.00', '0.20', '0.00', '0.00', '-0.20', '4.00', '4.00', '0.00', '0.00', '0.00', '0.00', NULL, 0, 0x04e83fabc6494c538e3d70f3ea1e031b, NULL, NULL, NULL, '2026-10-20 18:43:07.881674', '2026-10-20 18:43:07.881674'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c3c7a476a48a17e9cd82ddd428, 0x01a14c8ff30876da9be77f7076393059, 0x01a14c8ff30c75c6b3bef99b21380151, 'INV-2610-00005', '2026-10-25 15:52:52.887641', '2026-10-25', 0x11f1cab9efc2e801ab00bc542fed17aa, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 0x01a17386a31c75dfb6c82efa53e9ff79, NULL, 'completed', 'paid', 0, '2.00', '0.00', NULL, NULL, '0.00', '0.10', '0.00', '0.00', '-0.10', '2.00', '2.00', '0.00', '0.00', '8.00', '0.00', NULL, 0, 0x7a469dae9ac34ae2adb691674878c177, NULL, NULL, NULL, '2026-10-25 15:52:52.913063', '2026-10-25 15:52:52.913063'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cb7caa7227ac4c13cf122599af, 0x01a14c8ff30876da9be77f7076393059, 0x01a14c8ff30c75c6b3bef99b21380151, 'INV-2610-00006', '2026-10-25 16:01:17.990243', '2026-10-25', 0x11f1cab9efc2e801ab00bc542fed17aa, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 0x01a17386a31c75dfb6c82efa53e9ff79, NULL, 'completed', 'paid', 0, '2.00', '0.00', NULL, NULL, '0.00', '0.10', '0.00', '0.00', '-0.10', '2.00', '2.00', '0.00', '0.00', '8.00', '0.00', NULL, 0, 0x24d8d1b1e499481f9aa09274d3c89c01, NULL, NULL, NULL, '2026-10-25 16:01:17.996594', '2026-10-25 16:01:17.996594'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cbf16c77768110a202cf51cead, 0x01a14c8ff30876da9be77f7076393059, 0x01a14c8ff30c75c6b3bef99b21380151, 'INV-2610-00007', '2026-10-25 16:01:47.879688', '2026-10-25', 0x11f1cab9efc2e801ab00bc542fed17aa, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 0x01a17386a31c75dfb6c82efa53e9ff79, NULL, 'completed', 'paid', 0, '2.00', '0.00', NULL, NULL, '0.00', '0.10', '0.00', '0.00', '-0.10', '2.00', '2.00', '0.00', '0.00', '31.00', '0.00', NULL, 0, 0xdec75e594c854cf4a7cf718cbe4d7783, NULL, NULL, NULL, '2026-10-25 16:01:47.887823', '2026-10-25 16:01:47.887823'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d0cf157236a45375e8a5272e85, 0x01a14c8ff30876da9be77f7076393059, 0x01a14c8ff30c75c6b3bef99b21380151, 'INV-2610-00008', '2026-10-25 16:07:06.770206', '2026-10-25', 0x11f1cab9efc2e801ab00bc542fed17aa, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 0x01a17386a31c75dfb6c82efa53e9ff79, NULL, 'completed', 'paid', 0, '2.00', '0.00', NULL, NULL, '0.00', '0.10', '0.00', '0.00', '-0.10', '2.00', '2.00', '0.00', '0.00', '0.00', '0.00', NULL, 0, 0x138543dd78f743819d3014a402285d1d, NULL, NULL, NULL, '2026-10-25 16:07:06.775122', '2026-10-25 16:07:06.775122');

--
-- Triggers `sales`
--
DELIMITER $$
CREATE TRIGGER `trg_sales_tg_del` BEFORE DELETE ON `sales` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_sales_tg_ins` BEFORE INSERT ON `sales` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_sales_tg_upd` BEFORE UPDATE ON `sales` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `sale_items`
--

CREATE TABLE `sale_items` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `sale_id` binary(16) NOT NULL,
  `line_no` smallint NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `product_name` varchar(200) NOT NULL,
  `variant_name` varchar(100) DEFAULT NULL,
  `sku` varchar(64) NOT NULL,
  `unit_id` binary(16) NOT NULL,
  `unit_short_name` varchar(10) NOT NULL,
  `unit_factor` decimal(18,6) NOT NULL DEFAULT '1.000000',
  `track_inventory` tinyint(1) NOT NULL DEFAULT '1',
  `quantity` decimal(18,3) NOT NULL,
  `base_quantity` decimal(18,3) NOT NULL,
  `returned_quantity` decimal(18,3) NOT NULL DEFAULT '0.000',
  `list_price` decimal(18,4) NOT NULL,
  `unit_price` decimal(18,4) NOT NULL,
  `price_overridden` tinyint(1) NOT NULL DEFAULT '0',
  `gross_amount` decimal(18,2) NOT NULL,
  `discount_type` varchar(10) DEFAULT NULL,
  `discount_value` decimal(18,4) DEFAULT NULL,
  `line_discount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `invoice_discount_alloc` decimal(18,2) NOT NULL DEFAULT '0.00',
  `taxable_amount` decimal(18,2) NOT NULL,
  `tax_rate_id` binary(16) DEFAULT NULL,
  `tax_rate_pct` decimal(7,4) NOT NULL DEFAULT '0.0000',
  `tax_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `line_total` decimal(18,2) NOT NULL,
  `unit_cost` decimal(18,4) NOT NULL DEFAULT '0.0000',
  `cost_total` decimal(18,2) NOT NULL DEFAULT '0.00'
) ;

--
-- Dumping data for table `sale_items`
--

INSERT INTO `sale_items` (`tenant_id`, `id`, `sale_id`, `line_no`, `variant_id`, `product_name`, `variant_name`, `sku`, `unit_id`, `unit_short_name`, `unit_factor`, `track_inventory`, `quantity`, `base_quantity`, `returned_quantity`, `list_price`, `unit_price`, `price_overridden`, `gross_amount`, `discount_type`, `discount_value`, `line_discount`, `invoice_discount_alloc`, `taxable_amount`, `tax_rate_id`, `tax_rate_pct`, `tax_amount`, `line_total`, `unit_cost`, `cost_total`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9ff28e7600b632f67be7c07aad, 0x01a14d9ff28c702f800284db35995a81, 1, 0x01a14c91a5fa72269dd24dd5a08717cf, 'Ata 800', NULL, 'Ata-001', 0x01a14c8ff2f9761eb9f638875ad3ce54, 'kg', '1.000000', 0, '1.000', '1.000', '0.000', '2.0000', '2.0000', 0, '2.00', NULL, NULL, '0.00', '0.00', '2.00', 0x01a14c8ff2ed71d6a7c4621fce01d7eb, '5.0000', '0.10', '2.10', '0.0000', '0.00'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14da336cc700bb5e83fe7b92860c5, 0x01a14da336ca723dac4fec5f7d9b1aeb, 1, 0x01a14c91a5fa72269dd24dd5a08717cf, 'Ata 800', NULL, 'Ata-001', 0x01a14c8ff2f9761eb9f638875ad3ce54, 'kg', '1.000000', 0, '3.000', '3.000', '0.000', '2.0000', '2.0000', 0, '6.00', 'percent', '10.0000', '0.60', '0.00', '5.40', 0x01a14c8ff2ed71d6a7c4621fce01d7eb, '5.0000', '0.27', '5.67', '0.0000', '0.00'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d33c7574c69c99233c18ca09c6, 0x01a151d33c73715180219c1cedd1bb46, 1, 0x01a14c91a5fa72269dd24dd5a08717cf, 'Ata 800', NULL, 'Ata-001', 0x01a14c8ff2f9761eb9f638875ad3ce54, 'kg', '1.000000', 0, '2.000', '2.000', '1.000', '2.0000', '2.0000', 0, '4.00', NULL, NULL, '0.00', '0.00', '4.00', 0x01a14c8ff2ed71d6a7c4621fce01d7eb, '5.0000', '0.20', '4.20', '0.0000', '0.00'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9fd9e873aba80bdf73d473feca, 0x01a15a9fd9e6747db53070ca2ec2e354, 1, 0x01a14c91a5fa72269dd24dd5a08717cf, 'Ata 800', NULL, 'Ata-001', 0x01a14c8ff2f9761eb9f638875ad3ce54, 'kg', '1.000000', 0, '2.000', '2.000', '0.000', '2.0000', '2.0000', 0, '4.00', NULL, NULL, '0.00', '0.00', '4.00', 0x01a14c8ff2ed71d6a7c4621fce01d7eb, '5.0000', '0.20', '4.20', '0.0000', '0.00'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c3c7ad708ab74b1eb63be36d68, 0x01a173c3c7a476a48a17e9cd82ddd428, 1, 0x01a14c91a5fa72269dd24dd5a08717cf, 'Ata 800', NULL, 'Ata-001', 0x01a14c8ff2f9761eb9f638875ad3ce54, 'kg', '1.000000', 0, '1.000', '1.000', '0.000', '2.0000', '2.0000', 0, '2.00', NULL, NULL, '0.00', '0.00', '2.00', 0x01a14c8ff2ed71d6a7c4621fce01d7eb, '5.0000', '0.10', '2.10', '0.0000', '0.00'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cb7cab75b1972c7ef148471b8e, 0x01a173cb7caa7227ac4c13cf122599af, 1, 0x01a14c91a5fa72269dd24dd5a08717cf, 'Ata 800', NULL, 'Ata-001', 0x01a14c8ff2f9761eb9f638875ad3ce54, 'kg', '1.000000', 0, '1.000', '1.000', '0.000', '2.0000', '2.0000', 0, '2.00', NULL, NULL, '0.00', '0.00', '2.00', 0x01a14c8ff2ed71d6a7c4621fce01d7eb, '5.0000', '0.10', '2.10', '0.0000', '0.00'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173cbf16f70b290512806280fab9d, 0x01a173cbf16c77768110a202cf51cead, 1, 0x01a14c91a5fa72269dd24dd5a08717cf, 'Ata 800', NULL, 'Ata-001', 0x01a14c8ff2f9761eb9f638875ad3ce54, 'kg', '1.000000', 0, '1.000', '1.000', '0.000', '2.0000', '2.0000', 0, '2.00', NULL, NULL, '0.00', '0.00', '2.00', 0x01a14c8ff2ed71d6a7c4621fce01d7eb, '5.0000', '0.10', '2.10', '0.0000', '0.00'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d0cf16737a8950438aa28b2252, 0x01a173d0cf157236a45375e8a5272e85, 1, 0x01a14c91a5fa72269dd24dd5a08717cf, 'Ata 800', NULL, 'Ata-001', 0x01a14c8ff2f9761eb9f638875ad3ce54, 'kg', '1.000000', 0, '1.000', '1.000', '0.000', '2.0000', '2.0000', 0, '2.00', NULL, NULL, '0.00', '0.00', '2.00', 0x01a14c8ff2ed71d6a7c4621fce01d7eb, '5.0000', '0.10', '2.10', '0.0000', '0.00');

--
-- Triggers `sale_items`
--
DELIMITER $$
CREATE TRIGGER `trg_sale_items_tg_del` BEFORE DELETE ON `sale_items` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_sale_items_tg_ins` BEFORE INSERT ON `sale_items` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_sale_items_tg_upd` BEFORE UPDATE ON `sale_items` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `sale_returns`
--

CREATE TABLE `sale_returns` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `return_no` varchar(40) NOT NULL,
  `sale_id` binary(16) NOT NULL,
  `customer_id` binary(16) NOT NULL,
  `return_date` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `business_date` date NOT NULL,
  `reason` varchar(500) NOT NULL,
  `refund_mode` varchar(20) NOT NULL,
  `subtotal` decimal(18,2) NOT NULL,
  `tax_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `total` decimal(18,2) NOT NULL,
  `applied_to_due` decimal(18,2) NOT NULL DEFAULT '0.00',
  `refunded_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `credited_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `cost_total` decimal(18,2) NOT NULL DEFAULT '0.00',
  `status` varchar(20) NOT NULL DEFAULT 'posted',
  `approved_by` binary(16) DEFAULT NULL,
  `idempotency_key` binary(16) NOT NULL,
  `created_by` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `sale_returns`
--

INSERT INTO `sale_returns` (`tenant_id`, `id`, `branch_id`, `return_no`, `sale_id`, `customer_id`, `return_date`, `business_date`, `reason`, `refund_mode`, `subtotal`, `tax_total`, `total`, `applied_to_due`, `refunded_amount`, `credited_amount`, `cost_total`, `status`, `approved_by`, `idempotency_key`, `created_by`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1521a069874d2a5190fdfcd9f0412, 0x01a14c8ff30876da9be77f7076393059, 'RET-2610-00001', 0x01a151d33c73715180219c1cedd1bb46, 0x11f1cab9efc2e801ab00bc542fed17aa, '2026-10-19 02:59:59.772675', '2026-10-19', 'test', 'cash', '2.00', '0.10', '2.10', '0.00', '2.10', '0.00', '0.00', 'posted', NULL, 0xb75b5fdc9589401ba6bad46ddeba1a6a, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-19 02:59:59.772675');

--
-- Triggers `sale_returns`
--
DELIMITER $$
CREATE TRIGGER `trg_sale_returns_tg_del` BEFORE DELETE ON `sale_returns` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_sale_returns_tg_ins` BEFORE INSERT ON `sale_returns` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_sale_returns_tg_upd` BEFORE UPDATE ON `sale_returns` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `sale_return_items`
--

CREATE TABLE `sale_return_items` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `return_id` binary(16) NOT NULL,
  `sale_item_id` binary(16) NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `quantity` decimal(18,3) NOT NULL,
  `base_quantity` decimal(18,3) NOT NULL,
  `item_condition` varchar(10) NOT NULL,
  `location_id` binary(16) NOT NULL,
  `taxable_amount` decimal(18,2) NOT NULL,
  `tax_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `line_total` decimal(18,2) NOT NULL,
  `unit_cost` decimal(18,4) NOT NULL
) ;

--
-- Dumping data for table `sale_return_items`
--

INSERT INTO `sale_return_items` (`tenant_id`, `id`, `return_id`, `sale_item_id`, `variant_id`, `quantity`, `base_quantity`, `item_condition`, `location_id`, `taxable_amount`, `tax_amount`, `line_total`, `unit_cost`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a1521a069b77bc87262db83d102c2c, 0x01a1521a069874d2a5190fdfcd9f0412, 0x01a151d33c7574c69c99233c18ca09c6, 0x01a14c91a5fa72269dd24dd5a08717cf, '1.000', '1.000', 'restock', 0x01a14c8ff30c75c6b3bef99b21380151, '2.00', '0.10', '2.10', '0.0000');

--
-- Triggers `sale_return_items`
--
DELIMITER $$
CREATE TRIGGER `trg_sale_return_items_tg_del` BEFORE DELETE ON `sale_return_items` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_sale_return_items_tg_ins` BEFORE INSERT ON `sale_return_items` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_sale_return_items_tg_upd` BEFORE UPDATE ON `sale_return_items` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `stock_adjustments`
--

CREATE TABLE `stock_adjustments` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `location_id` binary(16) NOT NULL,
  `adjustment_no` varchar(40) DEFAULT NULL,
  `business_date` date NOT NULL,
  `reason` varchar(20) NOT NULL,
  `note` text,
  `status` varchar(20) NOT NULL DEFAULT 'draft',
  `total_value` decimal(18,2) NOT NULL DEFAULT '0.00',
  `posted_at` datetime(6) DEFAULT NULL,
  `posted_by` binary(16) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL
) ;

--
-- Triggers `stock_adjustments`
--
DELIMITER $$
CREATE TRIGGER `trg_stock_adjustments_tg_del` BEFORE DELETE ON `stock_adjustments` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_adjustments_tg_ins` BEFORE INSERT ON `stock_adjustments` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_adjustments_tg_upd` BEFORE UPDATE ON `stock_adjustments` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `stock_adjustment_items`
--

CREATE TABLE `stock_adjustment_items` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `adjustment_id` binary(16) NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `quantity` decimal(18,3) NOT NULL,
  `unit_cost` decimal(18,4) DEFAULT NULL,
  `note` varchar(255) DEFAULT NULL
) ;

--
-- Triggers `stock_adjustment_items`
--
DELIMITER $$
CREATE TRIGGER `trg_stock_adjustment_items_tg_del` BEFORE DELETE ON `stock_adjustment_items` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_adjustment_items_tg_ins` BEFORE INSERT ON `stock_adjustment_items` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_adjustment_items_tg_upd` BEFORE UPDATE ON `stock_adjustment_items` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `stock_balances`
--

CREATE TABLE `stock_balances` (
  `tenant_id` binary(16) NOT NULL,
  `location_id` binary(16) NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `quantity` decimal(18,3) NOT NULL DEFAULT '0.000',
  `avg_cost` decimal(18,4) NOT NULL DEFAULT '0.0000',
  `last_movement_at` datetime(6) DEFAULT NULL,
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `stock_balances`
--

INSERT INTO `stock_balances` (`tenant_id`, `location_id`, `variant_id`, `quantity`, `avg_cost`, `last_movement_at`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff30c75c6b3bef99b21380151, 0x01a173e1bc2f72d38cbf3f359dbef54a, '1.000', '4.0000', '2026-10-25 16:26:33.795713', '2026-10-25 16:26:33.795713');

--
-- Triggers `stock_balances`
--
DELIMITER $$
CREATE TRIGGER `trg_stock_balances_tg_del` BEFORE DELETE ON `stock_balances` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_balances_tg_ins` BEFORE INSERT ON `stock_balances` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_balances_tg_upd` BEFORE UPDATE ON `stock_balances` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `stock_counts`
--

CREATE TABLE `stock_counts` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `count_no` varchar(40) DEFAULT NULL,
  `branch_id` binary(16) NOT NULL,
  `location_id` binary(16) NOT NULL,
  `category_id` binary(16) DEFAULT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'draft',
  `note` varchar(500) DEFAULT NULL,
  `business_date` date NOT NULL,
  `variance_value` decimal(18,2) NOT NULL DEFAULT '0.00',
  `snapshot_at` datetime(6) DEFAULT NULL,
  `posted_at` datetime(6) DEFAULT NULL,
  `posted_by` binary(16) DEFAULT NULL,
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL
) ;

--
-- Triggers `stock_counts`
--
DELIMITER $$
CREATE TRIGGER `trg_stock_counts_tg_del` BEFORE DELETE ON `stock_counts` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_counts_tg_ins` BEFORE INSERT ON `stock_counts` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_counts_tg_upd` BEFORE UPDATE ON `stock_counts` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `stock_count_items`
--

CREATE TABLE `stock_count_items` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `count_id` binary(16) NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `system_qty` decimal(18,3) NOT NULL,
  `counted_qty` decimal(18,3) DEFAULT NULL,
  `variance_qty` decimal(18,3) GENERATED ALWAYS AS ((`counted_qty` - `system_qty`)) STORED,
  `unit_cost` decimal(18,4) DEFAULT NULL
) ;

--
-- Triggers `stock_count_items`
--
DELIMITER $$
CREATE TRIGGER `trg_stock_count_items_tg_del` BEFORE DELETE ON `stock_count_items` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_count_items_tg_ins` BEFORE INSERT ON `stock_count_items` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_count_items_tg_upd` BEFORE UPDATE ON `stock_count_items` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `stock_locations`
--

CREATE TABLE `stock_locations` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `code` varchar(20) NOT NULL,
  `name` varchar(100) NOT NULL,
  `location_type` varchar(20) NOT NULL DEFAULT 'store',
  `is_sellable` tinyint(1) NOT NULL DEFAULT '1',
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `stock_locations`
--

INSERT INTO `stock_locations` (`tenant_id`, `id`, `branch_id`, `code`, `name`, `location_type`, `is_sellable`, `status`, `created_at`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff30c75c6b3bef99b21380151, 0x01a14c8ff30876da9be77f7076393059, 'MAIN', 'Shop Floor', 'store', 1, 'active', '2026-10-18 01:11:04.718043', '2026-10-18 01:11:04.718043'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff30c75c6b3bef99c68074ca4, 0x01a14c8ff30876da9be77f7076393059, 'DAMAGED', 'Damaged', 'damaged', 0, 'active', '2026-10-18 01:11:04.719596', '2026-10-18 01:11:04.719596');

--
-- Triggers `stock_locations`
--
DELIMITER $$
CREATE TRIGGER `trg_stock_locations_tg_del` BEFORE DELETE ON `stock_locations` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_locations_tg_ins` BEFORE INSERT ON `stock_locations` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_locations_tg_upd` BEFORE UPDATE ON `stock_locations` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `stock_movements`
--

CREATE TABLE `stock_movements` (
  `tenant_id` binary(16) NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `location_id` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `id` binary(16) NOT NULL,
  `branch_id` binary(16) NOT NULL,
  `movement_type` varchar(20) NOT NULL,
  `quantity` decimal(18,3) NOT NULL,
  `unit_cost` decimal(18,4) NOT NULL,
  `total_cost` decimal(18,2) NOT NULL,
  `balance_qty_after` decimal(18,3) NOT NULL,
  `avg_cost_after` decimal(18,4) NOT NULL,
  `reference_type` varchar(30) NOT NULL,
  `reference_id` binary(16) NOT NULL,
  `reference_line_id` binary(16) DEFAULT NULL,
  `reference_no` varchar(40) DEFAULT NULL,
  `reason` varchar(255) DEFAULT NULL,
  `batch_id` binary(16) DEFAULT NULL,
  `business_date` date NOT NULL,
  `created_by` binary(16) DEFAULT NULL
) ;

--
-- Dumping data for table `stock_movements`
--

INSERT INTO `stock_movements` (`tenant_id`, `variant_id`, `location_id`, `created_at`, `id`, `branch_id`, `movement_type`, `quantity`, `unit_cost`, `total_cost`, `balance_qty_after`, `avg_cost_after`, `reference_type`, `reference_id`, `reference_line_id`, `reference_no`, `reason`, `batch_id`, `business_date`, `created_by`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173e1bc2f72d38cbf3f359dbef54a, 0x01a14c8ff30c75c6b3bef99b21380151, '2026-10-25 16:26:33.691383', 0x01a173e29c6c75fba679354cc8ae6ba1, 0x01a14c8ff30876da9be77f7076393059, 'purchase_receipt', '1.000', '4.0000', '4.00', '1.000', '4.0000', 'goods_receipt', 0x01a173e29c65768cb83fc880e0178154, 0x01a173e299b370b593e7634198c2b81c, 'GRN-2610-00001', NULL, NULL, '2026-10-25', 0x01a14c8ff09475ea92a85fc5af151c9e);

--
-- Triggers `stock_movements`
--
DELIMITER $$
CREATE TRIGGER `trg_stock_movements_ao_del` BEFORE DELETE ON `stock_movements` FOR EACH ROW BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_movements_ao_upd` BEFORE UPDATE ON `stock_movements` FOR EACH ROW BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_movements_tg_ins` BEFORE INSERT ON `stock_movements` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `stock_transfers`
--

CREATE TABLE `stock_transfers` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `transfer_no` varchar(40) DEFAULT NULL,
  `from_branch_id` binary(16) NOT NULL,
  `from_location_id` binary(16) NOT NULL,
  `to_branch_id` binary(16) NOT NULL,
  `to_location_id` binary(16) NOT NULL,
  `business_date` date NOT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'draft',
  `note` text,
  `total_value` decimal(18,2) NOT NULL DEFAULT '0.00',
  `loss_value` decimal(18,2) NOT NULL DEFAULT '0.00',
  `dispatched_at` datetime(6) DEFAULT NULL,
  `dispatched_by` binary(16) DEFAULT NULL,
  `received_at` datetime(6) DEFAULT NULL,
  `received_by` binary(16) DEFAULT NULL,
  `cancelled_at` datetime(6) DEFAULT NULL,
  `cancelled_by` binary(16) DEFAULT NULL,
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL
) ;

--
-- Triggers `stock_transfers`
--
DELIMITER $$
CREATE TRIGGER `trg_stock_transfers_tg_del` BEFORE DELETE ON `stock_transfers` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_transfers_tg_ins` BEFORE INSERT ON `stock_transfers` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_transfers_tg_upd` BEFORE UPDATE ON `stock_transfers` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `stock_transfer_items`
--

CREATE TABLE `stock_transfer_items` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `transfer_id` binary(16) NOT NULL,
  `variant_id` binary(16) NOT NULL,
  `quantity_sent` decimal(18,3) NOT NULL,
  `quantity_received` decimal(18,3) DEFAULT NULL,
  `unit_cost` decimal(18,4) DEFAULT NULL
) ;

--
-- Triggers `stock_transfer_items`
--
DELIMITER $$
CREATE TRIGGER `trg_stock_transfer_items_tg_del` BEFORE DELETE ON `stock_transfer_items` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_transfer_items_tg_ins` BEFORE INSERT ON `stock_transfer_items` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_stock_transfer_items_tg_upd` BEFORE UPDATE ON `stock_transfer_items` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `subscriptions`
--

CREATE TABLE `subscriptions` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `plan_id` binary(16) NOT NULL,
  `status` varchar(20) NOT NULL,
  `billing_cycle` varchar(20) NOT NULL DEFAULT 'monthly',
  `price_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `currency_code` char(3) NOT NULL DEFAULT 'BDT',
  `trial_ends_at` datetime(6) DEFAULT NULL,
  `current_period_start` datetime(6) DEFAULT NULL,
  `current_period_end` datetime(6) DEFAULT NULL,
  `grace_ends_at` datetime(6) DEFAULT NULL,
  `read_only_ends_at` datetime(6) DEFAULT NULL,
  `cancel_at_period_end` tinyint(1) NOT NULL DEFAULT '0',
  `cancelled_at` datetime(6) DEFAULT NULL,
  `pending_plan_id` binary(16) DEFAULT NULL,
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `subscriptions`
--

INSERT INTO `subscriptions` (`tenant_id`, `id`, `plan_id`, `status`, `billing_cycle`, `price_amount`, `currency_code`, `trial_ends_at`, `current_period_start`, `current_period_end`, `grace_ends_at`, `read_only_ends_at`, `cancel_at_period_end`, `cancelled_at`, `pending_plan_id`, `version`, `created_at`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3c3747bad2f2d76c13b5001, 0x019a0000000070008000000000000001, 'trialing', 'monthly', '0.00', 'BDT', '2026-11-01 01:11:04.901771', '2026-10-18 01:11:04.901771', '2026-11-01 01:11:04.901771', NULL, NULL, 0, NULL, NULL, 1, '2026-10-18 01:11:04.901771', '2026-10-18 01:11:04.901771');

--
-- Triggers `subscriptions`
--
DELIMITER $$
CREATE TRIGGER `trg_subscriptions_tg_del` BEFORE DELETE ON `subscriptions` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_subscriptions_tg_ins` BEFORE INSERT ON `subscriptions` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_subscriptions_tg_upd` BEFORE UPDATE ON `subscriptions` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `subscription_events`
--

CREATE TABLE `subscription_events` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `subscription_id` binary(16) NOT NULL,
  `event_type` varchar(40) NOT NULL,
  `from_status` varchar(20) DEFAULT NULL,
  `to_status` varchar(20) DEFAULT NULL,
  `from_plan_id` binary(16) DEFAULT NULL,
  `to_plan_id` binary(16) DEFAULT NULL,
  `actor_type` varchar(20) NOT NULL,
  `actor_id` binary(16) DEFAULT NULL,
  `reason` varchar(500) DEFAULT NULL,
  `metadata` json NOT NULL DEFAULT (json_object()),
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `subscription_events`
--

INSERT INTO `subscription_events` (`tenant_id`, `id`, `subscription_id`, `event_type`, `from_status`, `to_status`, `from_plan_id`, `to_plan_id`, `actor_type`, `actor_id`, `reason`, `metadata`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff3c771949e081bc15c7ef923, 0x01a14c8ff3c3747bad2f2d76c13b5001, 'trial_started', NULL, 'trialing', NULL, 0x019a0000000070008000000000000001, 'tenant_user', 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, '{}', '2026-10-18 01:11:04.905743');

--
-- Triggers `subscription_events`
--
DELIMITER $$
CREATE TRIGGER `trg_subscription_events_ao_del` BEFORE DELETE ON `subscription_events` FOR EACH ROW BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_subscription_events_ao_upd` BEFORE UPDATE ON `subscription_events` FOR EACH ROW BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_subscription_events_tg_ins` BEFORE INSERT ON `subscription_events` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `subscription_invoices`
--

CREATE TABLE `subscription_invoices` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `invoice_no` varchar(30) NOT NULL,
  `plan_id` binary(16) NOT NULL,
  `billing_cycle` varchar(20) NOT NULL,
  `period_start` datetime(6) NOT NULL,
  `period_end` datetime(6) NOT NULL,
  `subtotal` decimal(18,2) NOT NULL,
  `discount_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `tax_amount` decimal(18,2) NOT NULL DEFAULT '0.00',
  `total` decimal(18,2) NOT NULL,
  `status` varchar(10) NOT NULL DEFAULT 'open',
  `coupon_id` binary(16) DEFAULT NULL,
  `paid_at` datetime(6) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `subscription_invoices`
--

INSERT INTO `subscription_invoices` (`tenant_id`, `id`, `invoice_no`, `plan_id`, `billing_cycle`, `period_start`, `period_end`, `subtotal`, `discount_amount`, `tax_amount`, `total`, `status`, `coupon_id`, `paid_at`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173704e22774cbd647e556fe18b0c, 'SUB-2026-000001', 0x019a0000000070008000000000000003, 'annual', '2026-10-25 14:21:41.903934', '2027-10-25 14:21:41.903934', '25398.00', '0.00', '0.00', '25398.00', 'open', NULL, NULL, '2026-10-25 14:21:42.308518'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d3e50d74639e2eaa3b65f40694, 'SUB-2026-000002', 0x019a0000000070008000000000000003, 'monthly', '2026-10-25 16:10:29.002853', '2026-11-25 16:10:29.002853', '2490.00', '0.00', '0.00', '2490.00', 'open', NULL, NULL, '2026-10-25 16:10:29.006506');

--
-- Triggers `subscription_invoices`
--
DELIMITER $$
CREATE TRIGGER `trg_subscription_invoices_tg_del` BEFORE DELETE ON `subscription_invoices` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_subscription_invoices_tg_ins` BEFORE INSERT ON `subscription_invoices` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_subscription_invoices_tg_upd` BEFORE UPDATE ON `subscription_invoices` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `subscription_payment_claims`
--

CREATE TABLE `subscription_payment_claims` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `invoice_id` binary(16) NOT NULL,
  `method` varchar(20) NOT NULL,
  `amount` decimal(18,2) NOT NULL,
  `transaction_ref` varchar(60) NOT NULL,
  `payer_account` varchar(60) DEFAULT NULL,
  `proof_file_id` binary(16) DEFAULT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'pending',
  `reviewed_by` binary(16) DEFAULT NULL,
  `reviewed_at` datetime(6) DEFAULT NULL,
  `rejection_reason` varchar(500) DEFAULT NULL,
  `submitted_by` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `subscription_payment_claims`
--

INSERT INTO `subscription_payment_claims` (`tenant_id`, `id`, `invoice_id`, `method`, `amount`, `transaction_ref`, `payer_account`, `proof_file_id`, `status`, `reviewed_by`, `reviewed_at`, `rejection_reason`, `submitted_by`, `created_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173708fde70619e13105784332a21, 0x01a173704e22774cbd647e556fe18b0c, 'bkash', '25398.00', '2255', '22', NULL, 'pending', NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 14:21:59.135184'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173d400b7775ab13620dc13f8c5f7, 0x01a173d3e50d74639e2eaa3b65f40694, 'bkash', '2490.00', '545454', NULL, NULL, 'pending', NULL, NULL, NULL, 0x01a14c8ff09475ea92a85fc5af151c9e, '2026-10-25 16:10:36.087637');

--
-- Triggers `subscription_payment_claims`
--
DELIMITER $$
CREATE TRIGGER `trg_subscription_payment_claims_tg_del` BEFORE DELETE ON `subscription_payment_claims` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_subscription_payment_claims_tg_ins` BEFORE INSERT ON `subscription_payment_claims` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_subscription_payment_claims_tg_upd` BEFORE UPDATE ON `subscription_payment_claims` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `suppliers`
--

CREATE TABLE `suppliers` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `code` varchar(20) NOT NULL,
  `name` varchar(150) NOT NULL,
  `contact_person` varchar(150) DEFAULT NULL,
  `phone` varchar(20) DEFAULT NULL,
  `email` varchar(254) DEFAULT NULL,
  `address` varchar(500) DEFAULT NULL,
  `opening_balance` decimal(18,2) NOT NULL DEFAULT '0.00',
  `current_balance` decimal(18,2) NOT NULL DEFAULT '0.00',
  `notes` text,
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `version` int NOT NULL DEFAULT '1',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL,
  `deleted_at` datetime(6) DEFAULT NULL
) ;

--
-- Dumping data for table `suppliers`
--

INSERT INTO `suppliers` (`tenant_id`, `id`, `code`, `name`, `contact_person`, `phone`, `email`, `address`, `opening_balance`, `current_balance`, `notes`, `status`, `version`, `created_at`, `updated_at`, `created_by`, `deleted_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151d68d0974f8bbb2203ca6b1e0e0, 'S000001', 'Mr Kashem', '01839707645', NULL, 'omarshohag93@gmail.com', 'Lemua, Feni', '100.00', '0.00', 'Hello the world', 'active', 1, '2026-10-19 01:46:17.738407', '2026-10-25 16:26:47.631938', 0x01a14c8ff09475ea92a85fc5af151c9e, NULL);

--
-- Triggers `suppliers`
--
DELIMITER $$
CREATE TRIGGER `trg_suppliers_tg_del` BEFORE DELETE ON `suppliers` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_suppliers_tg_ins` BEFORE INSERT ON `suppliers` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_suppliers_tg_upd` BEFORE UPDATE ON `suppliers` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `tax_rates`
--

CREATE TABLE `tax_rates` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `name` varchar(50) NOT NULL,
  `rate_pct` decimal(7,4) NOT NULL,
  `is_default` tinyint(1) NOT NULL DEFAULT '0',
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `tax_rates`
--

INSERT INTO `tax_rates` (`tenant_id`, `id`, `name`, `rate_pct`, `is_default`, `status`, `created_at`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2e9708e988ff9bfb894aaf2, 'No VAT', '0.0000', 1, 'active', '2026-10-18 01:11:04.683759', '2026-10-18 01:11:04.683759'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2ed71d6a7c4621fce01d7eb, 'VAT 5%', '5.0000', 0, 'active', '2026-10-18 01:11:04.685992', '2026-10-18 01:11:04.685992'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2ef706ca8c168888f27856b, 'VAT 7.5%', '7.5000', 0, 'active', '2026-10-18 01:11:04.688074', '2026-10-18 01:11:04.688074'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2f17723b164745f67181b67, 'VAT 10%', '10.0000', 0, 'active', '2026-10-18 01:11:04.689823', '2026-10-18 01:11:04.689823'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2f477c6a5ec7e26240406b3, 'VAT 15%', '15.0000', 0, 'active', '2026-10-18 01:11:04.692544', '2026-10-18 01:11:04.692544');

--
-- Triggers `tax_rates`
--
DELIMITER $$
CREATE TRIGGER `trg_tax_rates_tg_del` BEFORE DELETE ON `tax_rates` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_tax_rates_tg_ins` BEFORE INSERT ON `tax_rates` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_tax_rates_tg_upd` BEFORE UPDATE ON `tax_rates` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `tenants`
--

CREATE TABLE `tenants` (
  `id` binary(16) NOT NULL,
  `slug` varchar(63) NOT NULL,
  `name` varchar(200) NOT NULL,
  `legal_name` varchar(200) DEFAULT NULL,
  `business_type` varchar(20) NOT NULL DEFAULT 'retail',
  `status` varchar(20) NOT NULL DEFAULT 'provisioning',
  `country_code` char(2) NOT NULL DEFAULT 'BD',
  `currency_code` char(3) NOT NULL DEFAULT 'BDT',
  `timezone` varchar(64) NOT NULL DEFAULT 'Asia/Dhaka',
  `locale` varchar(10) NOT NULL DEFAULT 'en',
  `phone` varchar(20) DEFAULT NULL,
  `email` varchar(254) DEFAULT NULL,
  `address` varchar(500) DEFAULT NULL,
  `district` varchar(100) DEFAULT NULL,
  `bin_number` varchar(30) DEFAULT NULL,
  `trade_license_no` varchar(50) DEFAULT NULL,
  `logo_file_id` binary(16) DEFAULT NULL,
  `is_sample_data` tinyint(1) NOT NULL DEFAULT '0',
  `suspended_reason` varchar(500) DEFAULT NULL,
  `terminated_at` datetime(6) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `tenants`
--

INSERT INTO `tenants` (`id`, `slug`, `name`, `legal_name`, `business_type`, `status`, `country_code`, `currency_code`, `timezone`, `locale`, `phone`, `email`, `address`, `district`, `bin_number`, `trade_license_no`, `logo_file_id`, `is_sample_data`, `suspended_reason`, `terminated_at`, `created_at`, `updated_at`) VALUES
(0x00000000000070008000000000000000, 'platform', 'Platform', NULL, 'other', 'active', 'BD', 'BDT', 'Asia/Dhaka', 'en', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, '2026-10-18 01:05:38.636701', '2026-10-18 01:05:38.636701'),
(0x01a14c8ff2df764db434b8b65d8bac97, 'test-shop', 'Test Shop', NULL, 'grocery', 'active', 'BD', 'BDT', 'Asia/Dhaka', 'en', '+8801700000001', 'shop@test.local', NULL, 'Dhaka', NULL, NULL, NULL, 0, NULL, NULL, '2026-10-18 01:11:04.678959', '2026-10-18 01:11:04.909896');

-- --------------------------------------------------------

--
-- Table structure for table `tenant_feature_overrides`
--

CREATE TABLE `tenant_feature_overrides` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `feature_code` varchar(60) NOT NULL,
  `enabled` tinyint(1) DEFAULT NULL,
  `limit_value` bigint DEFAULT NULL,
  `expires_at` datetime(6) DEFAULT NULL,
  `reason` varchar(500) NOT NULL,
  `created_by` binary(16) NOT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ;

--
-- Triggers `tenant_feature_overrides`
--
DELIMITER $$
CREATE TRIGGER `trg_tenant_feature_overrides_tg_del` BEFORE DELETE ON `tenant_feature_overrides` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_tenant_feature_overrides_tg_ins` BEFORE INSERT ON `tenant_feature_overrides` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_tenant_feature_overrides_tg_upd` BEFORE UPDATE ON `tenant_feature_overrides` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `tenant_memberships`
--

CREATE TABLE `tenant_memberships` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `user_id` binary(16) NOT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `is_owner` tinyint(1) NOT NULL DEFAULT '0',
  `perms_version` int NOT NULL DEFAULT '1',
  `display_name` varchar(150) DEFAULT NULL,
  `pin_hash` varchar(255) DEFAULT NULL,
  `joined_at` datetime(6) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `created_by` binary(16) DEFAULT NULL
) ;

--
-- Dumping data for table `tenant_memberships`
--

INSERT INTO `tenant_memberships` (`tenant_id`, `id`, `user_id`, `status`, `is_owner`, `perms_version`, `display_name`, `pin_hash`, `joined_at`, `created_at`, `updated_at`, `created_by`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff39674da9a7c90acc3f1b94b, 0x01a14c8ff09475ea92a85fc5af151c9e, 'active', 1, 1, 'Test Owner', NULL, '2026-10-18 01:11:04.856846', '2026-10-18 01:11:04.856846', '2026-10-18 01:11:04.856846', 0x01a14c8ff09475ea92a85fc5af151c9e);

--
-- Triggers `tenant_memberships`
--
DELIMITER $$
CREATE TRIGGER `trg_tenant_memberships_tg_del` BEFORE DELETE ON `tenant_memberships` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_tenant_memberships_tg_ins` BEFORE INSERT ON `tenant_memberships` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_tenant_memberships_tg_upd` BEFORE UPDATE ON `tenant_memberships` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `tenant_settings`
--

CREATE TABLE `tenant_settings` (
  `tenant_id` binary(16) NOT NULL,
  `currency_decimals` tinyint NOT NULL DEFAULT '2',
  `cash_rounding` decimal(6,2) NOT NULL DEFAULT '1.00',
  `prices_include_tax` tinyint(1) NOT NULL DEFAULT '0',
  `is_vat_registered` tinyint(1) NOT NULL DEFAULT '0',
  `default_tax_rate_id` binary(16) DEFAULT NULL,
  `allow_negative_stock` tinyint(1) NOT NULL DEFAULT '0',
  `costing_method` varchar(20) NOT NULL DEFAULT 'weighted_average',
  `return_window_days` int NOT NULL DEFAULT '7',
  `cashier_max_discount_pct` decimal(7,4) NOT NULL DEFAULT '10.0000',
  `fiscal_year_start_month` tinyint NOT NULL DEFAULT '7',
  `receipt_paper` varchar(10) NOT NULL DEFAULT '80mm',
  `receipt_footer` varchar(500) DEFAULT NULL,
  `auto_print_receipt` tinyint(1) NOT NULL DEFAULT '1',
  `show_bengali_digits` tinyint(1) NOT NULL DEFAULT '0',
  `extra` json NOT NULL DEFAULT (json_object()),
  `version` int NOT NULL DEFAULT '1',
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
  `updated_by` binary(16) DEFAULT NULL
) ;

--
-- Dumping data for table `tenant_settings`
--

INSERT INTO `tenant_settings` (`tenant_id`, `currency_decimals`, `cash_rounding`, `prices_include_tax`, `is_vat_registered`, `default_tax_rate_id`, `allow_negative_stock`, `costing_method`, `return_window_days`, `cashier_max_discount_pct`, `fiscal_year_start_month`, `receipt_paper`, `receipt_footer`, `auto_print_receipt`, `show_bengali_digits`, `extra`, `version`, `updated_at`, `updated_by`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 2, '1.00', 0, 0, 0x01a14c8ff2e9708e988ff9bfb894aaf2, 0, 'weighted_average', 7, '10.0000', 7, '80mm', NULL, 1, 0, '{\"onboarding\": {\"completed\": [\"business\", \"regional\", \"branch\", \"invoice\", \"products\", \"staff\"], \"dismissed\": false}}', 1, '2026-10-20 18:42:24.137479', 0x01a14c8ff09475ea92a85fc5af151c9e);

--
-- Triggers `tenant_settings`
--
DELIMITER $$
CREATE TRIGGER `trg_tenant_settings_tg_del` BEFORE DELETE ON `tenant_settings` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_tenant_settings_tg_ins` BEFORE INSERT ON `tenant_settings` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_tenant_settings_tg_upd` BEFORE UPDATE ON `tenant_settings` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `units`
--

CREATE TABLE `units` (
  `tenant_id` binary(16) NOT NULL,
  `id` binary(16) NOT NULL,
  `name` varchar(50) NOT NULL,
  `short_name` varchar(10) NOT NULL,
  `allow_fraction` tinyint(1) NOT NULL DEFAULT '0',
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `units`
--

INSERT INTO `units` (`tenant_id`, `id`, `name`, `short_name`, `allow_fraction`, `status`, `created_at`, `updated_at`) VALUES
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2f677f693a223fc85d990ab, 'Piece', 'pcs', 0, 'active', '2026-10-18 01:11:04.696129', '2026-10-18 01:11:04.696129'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2f9761eb9f638875ad3ce54, 'Kilogram', 'kg', 1, 'active', '2026-10-18 01:11:04.697776', '2026-10-18 01:11:04.697776'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2fa7178bc5c649409248600, 'Gram', 'g', 1, 'active', '2026-10-18 01:11:04.699229', '2026-10-18 01:11:04.699229'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2fc74ce9f1b097630d399a6, 'Litre', 'L', 1, 'active', '2026-10-18 01:11:04.700614', '2026-10-18 01:11:04.700614'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2fd7514b92c7254b151bcb2, 'Millilitre', 'ml', 1, 'active', '2026-10-18 01:11:04.701967', '2026-10-18 01:11:04.701967'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff2ff70c7a449d8637b390825, 'Box', 'box', 0, 'active', '2026-10-18 01:11:04.703516', '2026-10-18 01:11:04.703516'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff30072a48e48e9a8b93e796e, 'Dozen', 'dz', 0, 'active', '2026-10-18 01:11:04.705041', '2026-10-18 01:11:04.705041'),
(0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff30272929ee885b7863c3533, 'Pack', 'pack', 0, 'active', '2026-10-18 01:11:04.706754', '2026-10-18 01:11:04.706754');

--
-- Triggers `units`
--
DELIMITER $$
CREATE TRIGGER `trg_units_tg_del` BEFORE DELETE ON `units` FOR EACH ROW BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_units_tg_ins` BEFORE INSERT ON `units` FOR EACH ROW BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;
DELIMITER $$
CREATE TRIGGER `trg_units_tg_upd` BEFORE UPDATE ON `units` FOR EACH ROW BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END
$$
DELIMITER ;

-- --------------------------------------------------------

--
-- Table structure for table `users`
--

CREATE TABLE `users` (
  `id` binary(16) NOT NULL,
  `full_name` varchar(150) NOT NULL,
  `phone` varchar(16) DEFAULT NULL,
  `email` varchar(254) DEFAULT NULL,
  `password_hash` varchar(255) NOT NULL,
  `status` varchar(20) NOT NULL DEFAULT 'active',
  `phone_verified_at` datetime(6) DEFAULT NULL,
  `email_verified_at` datetime(6) DEFAULT NULL,
  `locale` varchar(10) NOT NULL DEFAULT 'en',
  `mfa_totp_secret_enc` varbinary(255) DEFAULT NULL,
  `mfa_enabled` tinyint(1) NOT NULL DEFAULT '0',
  `failed_login_count` int NOT NULL DEFAULT '0',
  `locked_until` datetime(6) DEFAULT NULL,
  `last_login_at` datetime(6) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `updated_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ;

--
-- Dumping data for table `users`
--

INSERT INTO `users` (`id`, `full_name`, `phone`, `email`, `password_hash`, `status`, `phone_verified_at`, `email_verified_at`, `locale`, `mfa_totp_secret_enc`, `mfa_enabled`, `failed_login_count`, `locked_until`, `last_login_at`, `created_at`, `updated_at`) VALUES
(0x01a14c8ff09475ea92a85fc5af151c9e, 'Test Owner', '+8801700000001', 'owner@test.local', '$argon2id$v=19$m=19456,t=2,p=1$8eLrcLx5oHaCHaGK1U2qzQ$nHAnjK7HtLLX+CZmBo9hnZbbV5MVZFawdwwfakpIlHI', 'active', '2026-10-18 01:11:04.296158', NULL, 'en', NULL, 0, 0, NULL, '2026-10-25 15:51:44.511415', '2026-10-18 01:11:04.086583', '2026-10-25 15:51:44.511415');

-- --------------------------------------------------------

--
-- Table structure for table `user_sessions`
--

CREATE TABLE `user_sessions` (
  `id` binary(16) NOT NULL,
  `user_id` binary(16) NOT NULL,
  `tenant_id` binary(16) DEFAULT NULL,
  `family_id` binary(16) NOT NULL,
  `refresh_token_hash` binary(32) NOT NULL,
  `replaced_by_id` binary(16) DEFAULT NULL,
  `ip_address` varbinary(16) DEFAULT NULL,
  `user_agent` varchar(500) DEFAULT NULL,
  `expires_at` datetime(6) NOT NULL,
  `last_used_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
  `revoked_at` datetime(6) DEFAULT NULL,
  `revoked_reason` varchar(40) DEFAULT NULL,
  `created_at` datetime(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Dumping data for table `user_sessions`
--

INSERT INTO `user_sessions` (`id`, `user_id`, `tenant_id`, `family_id`, `refresh_token_hash`, `replaced_by_id`, `ip_address`, `user_agent`, `expires_at`, `last_used_at`, `revoked_at`, `revoked_reason`, `created_at`) VALUES
(0x01a14c8ff17074ddaee71ad0723ab032, 0x01a14c8ff09475ea92a85fc5af151c9e, NULL, 0x01a14c8ff17074ddaee71acfd4061784, 0xb8060b021ec9649475911f6a842a2d615d23650ecbdf05ce334b5336caf886a7, 0x01a14c8ff4a1717cbeeddaa6dfac381d, 0x7f000001, 'curl/7.81.0', '2026-11-17 01:11:04.306341', '2026-10-18 01:11:05.123588', '2026-10-18 01:11:05.123588', 'rotated', '2026-10-18 01:11:04.306341'),
(0x01a14c8ff4a1717cbeeddaa6dfac381d, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c8ff17074ddaee71acfd4061784, 0x6a93ce706ba49751a521cac7560f2fcc2a3d729f2553fa6bbf109b7e42cdcc6b, NULL, 0x7f000001, 'curl/7.81.0', '2026-11-17 01:11:05.122314', '2026-10-18 01:11:05.122314', NULL, NULL, '2026-10-18 01:11:05.122314'),
(0x01a14c900b59719d887fc9c574058d32, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c900b59719d887fc9c4150a4fcd, 0x331ea244c751fb842e118b722801155d3c03d71d824efef4c306317648277958, NULL, 0x7f000001, 'curl/7.81.0', '2026-11-17 01:11:10.937572', '2026-10-18 01:11:10.937572', NULL, NULL, '2026-10-18 01:11:10.937572'),
(0x01a14c908f9676bc94765b8f64e29424, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x631e52986ae6cbf54e9404f4751d16c058409e8e0ebb0bf8274175348e37c3ec, 0x01a14c98b8ac728da82386f58a261be1, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 01:11:44.791146', '2026-10-18 01:20:39.597574', '2026-10-18 01:20:39.597574', 'rotated', '2026-10-18 01:11:44.791146'),
(0x01a14c98b8ac728da82386f58a261be1, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x27c8676be635bd186612873533767c45944ab5643730c6e88218c8e85c462b81, 0x01a14c98c02b7064bcf3eac40a7135ec, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 01:20:39.596894', '2026-10-18 01:20:41.516912', '2026-10-18 01:20:41.516912', 'rotated', '2026-10-18 01:20:39.596894'),
(0x01a14c98c02b7064bcf3eac40a7135ec, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x03cb0b15352d9de7b6605ca11bae79a55a49c9f252f5660fb7385dce1680ef44, 0x01a14ca359467372ab5ccdb170f977a6, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 01:20:41.515994', '2026-10-18 01:32:16.071622', '2026-10-18 01:32:16.071622', 'rotated', '2026-10-18 01:20:41.515994'),
(0x01a14ca359467372ab5ccdb170f977a6, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x9a626655ac3b2ae35f52d47169b78c7ce8e269cb8617e2af251efd4e21e6a56c, 0x01a14ca35f1e75a1b25cd9bd924ec129, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 01:32:16.070847', '2026-10-18 01:32:17.567882', '2026-10-18 01:32:17.567882', 'rotated', '2026-10-18 01:32:16.070847'),
(0x01a14ca35f1e75a1b25cd9bd924ec129, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xca78c64c65d0b12b7ef7b3172ed651ce37b9b9be4973747255f710f033527c4a, 0x01a14cabe95c701cb6875c8e46c74e81, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 01:32:17.567340', '2026-10-18 01:41:37.245975', '2026-10-18 01:41:37.245975', 'rotated', '2026-10-18 01:32:17.567340'),
(0x01a14cabe95c701cb6875c8e46c74e81, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xa337f0d696aefbf28a49af4f9c6e5c329cf4d71510fcead20a1299f114631160, 0x01a14cbbaccf7187a25c57420393e45b, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 01:41:37.245025', '2026-10-18 01:58:50.323452', '2026-10-18 01:58:50.323452', 'rotated', '2026-10-18 01:41:37.245025'),
(0x01a14cbbaccf7187a25c57420393e45b, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x6c5c44f082ab460fe5b9df186f08482d96912371e395fdd3a94529257bd7327c, 0x01a14cbbb360700f9b8193875aee26eb, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 01:58:50.320787', '2026-10-18 01:58:52.001835', '2026-10-18 01:58:52.001835', 'rotated', '2026-10-18 01:58:50.320787'),
(0x01a14cbbb360700f9b8193875aee26eb, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x6a691e3197c550fcaf2ed5225f439043f31de8d2aea3374273875c898daf84e5, 0x01a14cc0be24713ca6810ce6023ecd0f, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 01:58:52.001203', '2026-10-18 02:04:22.439086', '2026-10-18 02:04:22.439086', 'rotated', '2026-10-18 01:58:52.001203'),
(0x01a14cc0be24713ca6810ce6023ecd0f, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x1e2b105a53a7f5187a276338f8df6571794392f4a14bccffb9fd67b3bc58f919, 0x01a14cd94d877676acbfac496eb92d0c, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 02:04:22.437405', '2026-10-18 02:31:12.070070', '2026-10-18 02:31:12.070070', 'rotated', '2026-10-18 02:04:22.437405'),
(0x01a14cd94d877676acbfac496eb92d0c, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x126cc601b323350910bb12571c08d7523cac0b9db3beb6f35f57a179c16b5267, 0x01a14cd9820c73fb862c90a32b7117cc, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 02:31:12.009442', '2026-10-18 02:31:25.452924', '2026-10-18 02:31:25.452924', 'rotated', '2026-10-18 02:31:12.009442'),
(0x01a14cd9820c73fb862c90a32b7117cc, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xcdc5eccf268937981d5c056f417d96d27d6a68e2d5cbb6f63f9a59913d3cf9ac, 0x01a14ce1fc3f750a9f6809bf35860b77, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 02:31:25.452327', '2026-10-18 02:40:41.024962', '2026-10-18 02:40:41.024962', 'rotated', '2026-10-18 02:31:25.452327'),
(0x01a14ce1fc3f750a9f6809bf35860b77, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xf55a30ea7c810da2bbc3fd2f1679efd31b1d80e3bf032dab31bb7a9ddf6ad8df, 0x01a14ce715af77ad856856c666b589d1, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 02:40:41.024107', '2026-10-18 02:46:15.216983', '2026-10-18 02:46:15.216983', 'rotated', '2026-10-18 02:40:41.024107'),
(0x01a14ce715af77ad856856c666b589d1, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xa90d6c152381fffcddbd45fbfeee87197dfb7a06d49952cb8d2083c5aef0e945, 0x01a14cec2496779e9bc725fed17a4973, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 02:46:15.215944', '2026-10-18 02:51:46.711994', '2026-10-18 02:51:46.711994', 'rotated', '2026-10-18 02:46:15.215944'),
(0x01a14cec2496779e9bc725fed17a4973, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xad1fec8805603b3bf7f1012dfae3bd5556d9c6867503a9dd59dd2193bb2149e6, 0x01a14cf1d3fe774280de240c6b72a3db, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 02:51:46.711280', '2026-10-18 02:57:59.295332', '2026-10-18 02:57:59.295332', 'rotated', '2026-10-18 02:51:46.711280'),
(0x01a14cf1d3fe774280de240c6b72a3db, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x5b8734d644d8f7eaa0a3d5151dce94ec7afdb197e76bb755592a3cb0516a8219, 0x01a14cf279ae71f889e332f10fad97b0, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 02:57:59.294971', '2026-10-18 02:58:41.710913', '2026-10-18 02:58:41.710913', 'rotated', '2026-10-18 02:57:59.294971'),
(0x01a14cf279ae71f889e332f10fad97b0, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x4b419a7522d33436b63836c2b3a55c73ba9e39c3430cf4c612f354a2d55b1745, 0x01a14cf28e95756d9827d0741faa3bd6, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 02:58:41.710588', '2026-10-18 02:58:47.062109', '2026-10-18 02:58:47.062109', 'rotated', '2026-10-18 02:58:41.710588'),
(0x01a14cf28e95756d9827d0741faa3bd6, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x2a759e2a282591d7248c4b7835b49dd06fe0e01dcc01cd5c07adccb46b2defd2, 0x01a14cf29e8b727fa14e5cce3b220215, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 02:58:47.061721', '2026-10-18 02:58:51.148468', '2026-10-18 02:58:51.148468', 'rotated', '2026-10-18 02:58:47.061721'),
(0x01a14cf29e8b727fa14e5cce3b220215, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xc3c7dd1ad237ba3b167a35b5158c6e26173c819022c1c8791be2dd757288a927, 0x01a14cf4602f742c9f40f295c54f7397, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 02:58:51.147815', '2026-10-18 03:00:46.255929', '2026-10-18 03:00:46.255929', 'rotated', '2026-10-18 02:58:51.147815'),
(0x01a14cf4602f742c9f40f295c54f7397, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xcd44c53ba7697c07d34dd556613695d23242e125a4b70e31cb1f5df6366b9a4d, 0x01a14cf512887146bfcaa9e49c539b47, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:00:46.255629', '2026-10-18 03:01:31.913365', '2026-10-18 03:01:31.913365', 'rotated', '2026-10-18 03:00:46.255629'),
(0x01a14cf512887146bfcaa9e49c539b47, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xc5bb6f817b4120b453a08338c1ac4fd3dbfe37ee057a5c1c42c5fe631b8db6df, 0x01a14cf5304f776e847f608168705189, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:01:31.913003', '2026-10-18 03:01:39.536005', '2026-10-18 03:01:39.536005', 'rotated', '2026-10-18 03:01:31.913003'),
(0x01a14cf5304f776e847f608168705189, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xc1c7dce09555a0a05d1d2a3bde637db4ae3d752ee9deadf3e6bfc21bf8c6070a, 0x01a14cf54c2d719a976318695c107525, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:01:39.535611', '2026-10-18 03:01:46.670414', '2026-10-18 03:01:46.670414', 'rotated', '2026-10-18 03:01:39.535611'),
(0x01a14cf54c2d719a976318695c107525, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xd6b213bbe0832f855f4795d48334387423c6a95c2e99f5db6990d79d845a5da0, 0x01a14cf56606774a82959aa6df6b9dc2, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:01:46.670057', '2026-10-18 03:01:53.286947', '2026-10-18 03:01:53.286947', 'rotated', '2026-10-18 03:01:46.670057'),
(0x01a14cf56606774a82959aa6df6b9dc2, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x85981d84d5d43a1af0bc86f8d3201fa7f6eb78701fdb22a0225182a42f011d8c, 0x01a14cf5f65f71fbb1d890f0d33a1e0b, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:01:53.286600', '2026-10-18 03:02:30.240465', '2026-10-18 03:02:30.240465', 'rotated', '2026-10-18 03:01:53.286600'),
(0x01a14cf5f65f71fbb1d890f0d33a1e0b, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x5f550caf5c1e242e0e1d5e2c7ce528a1dd56943713cc75b1cf57a5df845b5c87, 0x01a14cf5fafb762898e527cfd4415ddf, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:02:30.240070', '2026-10-18 03:02:31.419730', '2026-10-18 03:02:31.419730', 'rotated', '2026-10-18 03:02:30.240070'),
(0x01a14cf5fafb762898e527cfd4415ddf, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x05c899122fd75770bb6817f7edd55ceab574560cb88c8287a10cd147e6ab077f, 0x01a14cf6246375cfa79324ac712f8a91, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:02:31.419437', '2026-10-18 03:02:42.019614', '2026-10-18 03:02:42.019614', 'rotated', '2026-10-18 03:02:31.419437'),
(0x01a14cf6246375cfa79324ac712f8a91, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x093b8cd80d266b1698612800659bb5fa428df7332fbd1f83e1a124dbb96738ac, 0x01a14cf6cf717384b42ba9a2a0b29dcd, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:02:42.019302', '2026-10-18 03:03:25.810310', '2026-10-18 03:03:25.810310', 'rotated', '2026-10-18 03:02:42.019302'),
(0x01a14cf6cf717384b42ba9a2a0b29dcd, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xa2727fc92cba1c653937b6524eae5b7099a5091718a945c406064344633492ee, 0x01a14cf6e18f73f7812052e1fb22fb7e, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:03:25.809981', '2026-10-18 03:03:30.448538', '2026-10-18 03:03:30.448538', 'rotated', '2026-10-18 03:03:25.809981'),
(0x01a14cf6e18f73f7812052e1fb22fb7e, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0xdc06696a87157037d1d8d47ecbb66ca455750bb1d05d0f947d7af1a7391e7b3b, 0x01a14cf7d4ac753e93db0151b97ee477, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:03:30.448172', '2026-10-18 03:04:32.685277', '2026-10-18 03:04:32.685277', 'rotated', '2026-10-18 03:03:30.448172'),
(0x01a14cf7d4ac753e93db0151b97ee477, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x07916db59a624b408028a80ff474a503f8e2c3ca6175b74f7789f94e7ef37690, 0x01a14cf8405d76e0a56c7c8145238688, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:04:32.684872', '2026-10-18 03:05:00.254700', '2026-10-18 03:05:00.254700', 'rotated', '2026-10-18 03:04:32.684872'),
(0x01a14cf8405d76e0a56c7c8145238688, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14c908f9676bc94765b8e5ddb30d1, 0x17bd1027b4bd85c09c71ff007c30de0dc4199609e59eaed1097b62d810bcf0c2, NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 03:05:00.254043', '2026-10-18 03:05:00.254043', '2026-10-18 03:05:10.202776', 'reuse_detected', '2026-10-18 03:05:00.254043'),
(0x01a14d4ab32b77d3a157a634558c6cfb, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x2848ec190ab4a6488495ee160a5448ef43b21d1888d52ecfd8a22870af1fd68b, 0x01a14d84b79c74b1a85d096cd0e78f5a, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 04:35:03.597184', '2026-10-18 05:38:25.825063', '2026-10-18 05:38:25.825063', 'rotated', '2026-10-18 04:35:03.597184'),
(0x01a14d84b79c74b1a85d096cd0e78f5a, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x6ba4bda6774894bd04fb216189972fc7fb926d52695ee8a17aad3bcbcdb3f60a, 0x01a14d8688c970bca99b53c434a1b25f, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:38:25.822536', '2026-10-18 05:40:24.906104', '2026-10-18 05:40:24.906104', 'rotated', '2026-10-18 05:38:25.822536'),
(0x01a14d8688c970bca99b53c434a1b25f, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xb877e2d445e900a7f04dd2f5466f474980688cd82cb4ba0e0681c53b0d67f72e, 0x01a14d8705f3714da63bab5a289bb922, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:40:24.905609', '2026-10-18 05:40:56.950780', '2026-10-18 05:40:56.950780', 'rotated', '2026-10-18 05:40:24.905609'),
(0x01a14d8705f3714da63bab5a289bb922, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x08a860bdb2d7ccda412ff722d79c6cf4c7c16f63babf8f2f53c15b7b58e909b4, 0x01a14d877de170308ede1d65c8ada31a, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:40:56.948599', '2026-10-18 05:41:27.650127', '2026-10-18 05:41:27.650127', 'rotated', '2026-10-18 05:40:56.948599'),
(0x01a14d877de170308ede1d65c8ada31a, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xbf6d004b03163cfeef7e2f796831b7a859c98eed488c45993d4e3f3b88d0e684, 0x01a14d87ecd576f49e2bf80fe631f813, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:41:27.649729', '2026-10-18 05:41:56.054964', '2026-10-18 05:41:56.054964', 'rotated', '2026-10-18 05:41:27.649729'),
(0x01a14d87ecd576f49e2bf80fe631f813, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xa29805e3f2389c17c779527fcfa2c51f6e96c5c31e7c696034598d4b8fdca809, 0x01a14d88cf3975c592e3f8c5dbb9209c, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:41:56.054363', '2026-10-18 05:42:54.009763', '2026-10-18 05:42:54.009763', 'rotated', '2026-10-18 05:41:56.054363'),
(0x01a14d88cf3975c592e3f8c5dbb9209c, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xc977dcc268df879c10e11390656d10ead9aed12967c90ab079e3f37c9775eeb4, 0x01a14d88ed1b7021879d3e579855dcbb, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:42:54.009329', '2026-10-18 05:43:01.660742', '2026-10-18 05:43:01.660742', 'rotated', '2026-10-18 05:42:54.009329'),
(0x01a14d88ed1b7021879d3e579855dcbb, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x6b426562844044ab5c12638ca172bbb307f21eb9d05ba16253e1b6032aed1808, 0x01a14d88f40a7671b5f847fc091116b7, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:43:01.660104', '2026-10-18 05:43:03.435205', '2026-10-18 05:43:03.435205', 'rotated', '2026-10-18 05:43:01.660104'),
(0x01a14d88f40a7671b5f847fc091116b7, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x106c86da61558940c7ea11ae89fb7a17c3c2ed9c889aca519bdc2a537b8b2e44, 0x01a14d89000d7305a8dce34ee122934f, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:43:03.434790', '2026-10-18 05:43:07.573289', '2026-10-18 05:43:07.573289', 'rotated', '2026-10-18 05:43:03.434790'),
(0x01a14d89000d7305a8dce34ee122934f, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x26cbc08eb04d34f36eb22e6b99daced27059850ac5323a149d90b364141cbeff, 0x01a14d8a1d0d77be8f13eaa27066200e, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:43:06.509921', '2026-10-18 05:44:19.472494', '2026-10-18 05:44:19.472494', 'rotated', '2026-10-18 05:43:06.509921'),
(0x01a14d8a1d0d77be8f13eaa27066200e, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x54e8720e5251279b4a98e7cdb6c04189f65a5bde39fc4c7accce5a6103e3b1ad, 0x01a14d8a889174ce828e3ec6d9f0b31c, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:44:19.470855', '2026-10-18 05:44:46.994202', '2026-10-18 05:44:46.994202', 'rotated', '2026-10-18 05:44:19.470855'),
(0x01a14d8a889174ce828e3ec6d9f0b31c, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x529e5e7a58db93a0733594b015106d33211b72d4738ee528832866339104a66a, 0x01a14d8cefa770cb847764b147360200, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:44:46.993474', '2026-10-18 05:47:24.456157', '2026-10-18 05:47:24.456157', 'rotated', '2026-10-18 05:44:46.993474'),
(0x01a14d8cefa770cb847764b147360200, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x6346151029b915a662bf6e380c4dbff6b691a7285c9aabee34fb1b62c4d6c5ba, 0x01a14d8e9b5a7400bd13b2e12948d027, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:47:24.455658', '2026-10-18 05:49:13.947369', '2026-10-18 05:49:13.947369', 'rotated', '2026-10-18 05:47:24.455658'),
(0x01a14d8e9b5a7400bd13b2e12948d027, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xe54085880a27fc629443cc37bb1c8a693ddab835a9e53fa8862aea6eea061111, 0x01a14d8ebd2172ccb7bd844d5129c3c7, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:49:13.946955', '2026-10-18 05:49:22.594515', '2026-10-18 05:49:22.594515', 'rotated', '2026-10-18 05:49:13.946955'),
(0x01a14d8ebd2172ccb7bd844d5129c3c7, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x146a3bddc69b8135efc2246a7f1bc08495f95d411dace16329cf69a9c03a5b1f, 0x01a14d910c727362974406061e1cffee, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:49:22.594095', '2026-10-18 05:51:53.971973', '2026-10-18 05:51:53.971973', 'rotated', '2026-10-18 05:49:22.594095'),
(0x01a14d910c727362974406061e1cffee, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x5c204985feda7486aa4f4bd986ae324eff98e79bf55aea4011480d5b14a01e12, 0x01a14d94039c7440a9d2bc5a14712bb8, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:51:53.971295', '2026-10-18 05:55:08.316613', '2026-10-18 05:55:08.316613', 'rotated', '2026-10-18 05:51:53.971295'),
(0x01a14d94039c7440a9d2bc5a14712bb8, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x0df5df71019d8f425bef0d337e57b38d98456df671413c712e5fb9adb57c7cbb, 0x01a14d945d9c77ebb6cd03330e87ba48, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:55:08.316282', '2026-10-18 05:55:31.357479', '2026-10-18 05:55:31.357479', 'rotated', '2026-10-18 05:55:08.316282'),
(0x01a14d945d9c77ebb6cd03330e87ba48, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x9b5e83beafda5d9a65ccd88d342fa716ffc4fe90b14851096b11913bfc3abeb2, 0x01a14d960e43738d8c4f955a635a7a4e, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:55:31.357020', '2026-10-18 05:57:22.115908', '2026-10-18 05:57:22.115908', 'rotated', '2026-10-18 05:55:31.357020'),
(0x01a14d960e43738d8c4f955a635a7a4e, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x220a4f0d03ca183e9abd330f824161382c9239eb1957cd49dc3614168fa104c9, 0x01a14d97c54d778a89149c97d3f76f75, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:57:22.115542', '2026-10-18 05:59:14.510240', '2026-10-18 05:59:14.510240', 'rotated', '2026-10-18 05:57:22.115542'),
(0x01a14d97c54d778a89149c97d3f76f75, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x4384c4beaae41b4a3e0d57dc56b92d92c7fddb59bf19eb5f5a40ad0e9ee5b87d, 0x01a14d98ad4a77258a99b7a38dfcc100, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 05:59:14.509827', '2026-10-18 06:00:13.899088', '2026-10-18 06:00:13.899088', 'rotated', '2026-10-18 05:59:14.509827'),
(0x01a14d98ad4a77258a99b7a38dfcc100, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xc2b5be0eff7e63e876d18ea681a599d9c848a70a35ea7f0515298895e4d314b7, 0x01a14d9f33b571cba9a4f1e8fa53a7f8, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 06:00:13.898675', '2026-10-18 06:07:21.526387', '2026-10-18 06:07:21.526387', 'rotated', '2026-10-18 06:00:13.898675'),
(0x01a14d9e612477c1b66868d12e3da480, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d9e612477c1b66868d0f4a798a3, 0x4a0620fb93da76b773456897f1a6b9ac09272c7a3f7d612e9972f67544e0d0c6, NULL, 0x7f000001, 'curl/7.81.0', '2026-11-17 06:06:27.621155', '2026-10-18 06:06:27.621155', NULL, NULL, '2026-10-18 06:06:27.621155'),
(0x01a14d9f33b571cba9a4f1e8fa53a7f8, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xa60746a1e03d683aa051aa73aac6b1b87a237efb4b2f93a49e369dacfdfb6531, 0x01a151cd05f97231be8b711b2bb27282, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-17 06:07:21.525596', '2026-10-19 01:35:53.338497', '2026-10-19 01:35:53.338497', 'rotated', '2026-10-18 06:07:21.525596'),
(0x01a151cd05f97231be8b711b2bb27282, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xad577e8d1c3d4f381eea7f893c77a8fef1903f2d845151a75a637b399c118acf, 0x01a151cd878271809619ca31aed3c2d9, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:35:53.338063', '2026-10-19 01:36:26.500356', '2026-10-19 01:36:26.500356', 'rotated', '2026-10-19 01:35:53.338063'),
(0x01a151cd878271809619ca31aed3c2d9, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x158363ee4415b44d663aebc3889d1f93785636269e0af1392679d255a82aa743, 0x01a151ce30d2714dad888213432599d3, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:36:26.498992', '2026-10-19 01:37:09.843825', '2026-10-19 01:37:09.843825', 'rotated', '2026-10-19 01:36:26.498992'),
(0x01a151ce30d2714dad888213432599d3, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xa208547d74e48dc304d45eeb3afc9db1162caa08c0e7c0a83d20c95088901590, 0x01a151d74f8474a4b37fbe4b80ec9940, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:37:09.843153', '2026-10-19 01:47:07.525743', '2026-10-19 01:47:07.525743', 'rotated', '2026-10-19 01:37:09.843153'),
(0x01a151d74f8474a4b37fbe4b80ec9940, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x61c024bc59ad78d4fde2a8efcb516c477c1dc9f69d350219228e490fbc6fa817, 0x01a151dbf60876b5a71625dd121dfd93, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:47:07.525167', '2026-10-19 01:52:12.297768', '2026-10-19 01:52:12.297768', 'rotated', '2026-10-19 01:47:07.525167'),
(0x01a151dbf60876b5a71625dd121dfd93, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x73322f620fc2448895f7e347e9093060bd1dc01a3a70d4112e6a76d88ad5786b, 0x01a151de1b6871a3aa8e4c6eab703f62, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:52:12.296848', '2026-10-19 01:54:32.937882', '2026-10-19 01:54:32.937882', 'rotated', '2026-10-19 01:52:12.296848'),
(0x01a151de1b6871a3aa8e4c6eab703f62, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x8a0ba8e3cacdaa650b69dbba2463716fc3e10890777b6152807199cef13164a5, 0x01a151df4a6876389b8b626f5e73dbc7, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:54:32.936901', '2026-10-19 01:55:50.508013', '2026-10-19 01:55:50.508013', 'rotated', '2026-10-19 01:54:32.936901'),
(0x01a151df4a6876389b8b626f5e73dbc7, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xabb4e3f4c67b6b25883159f665266452549c0a6f8e37ad172ba700fd1413e2f6, 0x01a151dff8b674c7a490517da87fff49, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:55:50.505320', '2026-10-19 01:56:35.127634', '2026-10-19 01:56:35.127634', 'rotated', '2026-10-19 01:55:50.505320'),
(0x01a151dff8b674c7a490517da87fff49, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x3dfcba8e11865c4a6958fe9d531877e9b4dd36a449467b2d973ac58cf2f0c446, 0x01a151e0723d712dadee9266633fccd2, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:56:35.126642', '2026-10-19 01:57:06.238388', '2026-10-19 01:57:06.238388', 'rotated', '2026-10-19 01:56:35.126642'),
(0x01a151e0723d712dadee9266633fccd2, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x9d810220ad4931d11aebc283adf3364f490beb35789f02953102f80d71620cc6, 0x01a151e08cb275e39851e649b475f5f8, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:57:06.238001', '2026-10-19 01:57:14.010324', '2026-10-19 01:57:14.010324', 'rotated', '2026-10-19 01:57:06.238001'),
(0x01a151e08cb275e39851e649b475f5f8, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x518d96c4433ad8e5ab74ae08ae6a4e6c558115947523ff623c236a50d9f092ec, 0x01a151e13b8f73b6958f8bb802137345, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:57:13.010770', '2026-10-19 01:57:57.775858', '2026-10-19 01:57:57.775858', 'rotated', '2026-10-19 01:57:13.010770'),
(0x01a151e13b8f73b6958f8bb802137345, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x8b45c8589eb04e9cbfb6c166d4cabac1ae6499e46b904b7ee386ea3acbfebb45, 0x01a151e1650676d19223d1e0f7361a63, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:57:57.775502', '2026-10-19 01:58:08.391633', '2026-10-19 01:58:08.391633', 'rotated', '2026-10-19 01:57:57.775502'),
(0x01a151e1650676d19223d1e0f7361a63, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x025617d360e8f009183ed87a48a0c424df9171118e411f303fd4eeb4193329d2, 0x01a151e19ffb76bbb36e9832ca3cf63e, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:58:08.391055', '2026-10-19 01:58:23.485194', '2026-10-19 01:58:23.485194', 'rotated', '2026-10-19 01:58:08.391055'),
(0x01a151e19ffb76bbb36e9832ca3cf63e, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x6b25422d85fbb73dc6e97d86e54ccd8878553d162a48390fa4ed35eef296c1b7, 0x01a151e28e6a74bdaf69cd8bc3475df8, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:58:23.484421', '2026-10-19 01:59:24.523571', '2026-10-19 01:59:24.523571', 'rotated', '2026-10-19 01:58:23.484421'),
(0x01a151e28e6a74bdaf69cd8bc3475df8, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0x1fda9a276cd5cb048ea55e8e60b2a370b6155491c40cf80082eff502da9a4206, 0x01a151e2f83f74ed945031ca9d436adb, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:59:24.522934', '2026-10-19 01:59:51.616240', '2026-10-19 01:59:51.616240', 'rotated', '2026-10-19 01:59:24.522934'),
(0x01a151e2f83f74ed945031ca9d436adb, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a14d4ab32b77d3a157a633e18641ef, 0xe14ed1b9df5e55dce004e5fe1a32bda4501d5eca25b030b62a6844ab72e77329, NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 01:59:51.615736', '2026-10-19 01:59:51.615736', '2026-10-19 01:59:55.767353', 'reuse_detected', '2026-10-19 01:59:51.615736'),
(0x01a151ea337177108cecb835cfb53628, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a151ea337177108cecb83493877b62, 0x612f13fce2c3a3925a3041fe45c51a79c874eaf80ff2d2497052c64f68f75919, NULL, 0x7f000001, 'curl/7.81.0', '2026-11-18 02:07:45.521668', '2026-10-19 02:07:45.521668', NULL, NULL, '2026-10-19 02:07:45.521668'),
(0x01a15204f9f7737d93cfe56d453c5b20, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0x89ce899e7e671e07921ee8a8e2c5bf8d64797d4f7e722f14f5dd01b4969a2084, 0x01a152171a9a7407a90bf9eeb6fc46b4, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 02:37:00.279454', '2026-10-19 02:56:48.287228', '2026-10-19 02:56:48.287228', 'rotated', '2026-10-19 02:37:00.279454'),
(0x01a152171a9a7407a90bf9eeb6fc46b4, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0x815a9c119aa8ab6beaff101f0c792afd13fcfab4c043828e105257e665c355cd, 0x01a152171fa2702daf7c15e05efb8988, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 02:56:48.284571', '2026-10-19 02:56:49.571128', '2026-10-19 02:56:49.571128', 'rotated', '2026-10-19 02:56:48.284571'),
(0x01a152171fa2702daf7c15e05efb8988, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0x8f27737ea86628530ffaadc2bd9937f305d78f2784066d5f50bad9cbd296c10e, 0x01a15218db59712e87a228f7e4be18d3, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 02:56:49.570447', '2026-10-19 02:58:43.162147', '2026-10-19 02:58:43.162147', 'rotated', '2026-10-19 02:56:49.570447'),
(0x01a15218db59712e87a228f7e4be18d3, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0x3226d225dc1e72576c9d1189345434370fe00977ba60098395cde77f695badd8, 0x01a1521cb86971ba8f7a01a9e8439f78, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 02:58:43.161523', '2026-10-19 03:02:56.362748', '2026-10-19 03:02:56.362748', 'rotated', '2026-10-19 02:58:43.161523'),
(0x01a1521cb86971ba8f7a01a9e8439f78, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0x535b0712bac3140e6b626bf3117735e823dc51261e4233f3b66e20896c4fa6c0, 0x01a15561d7d976b3a8f30caa16661e7f, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 03:02:56.361990', '2026-10-19 18:17:18.164025', '2026-10-19 18:17:18.164025', 'rotated', '2026-10-19 03:02:56.361990'),
(0x01a15561d7d976b3a8f30caa16661e7f, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0x1ea88d7e247dac0c7bb28a4ef8e5018620062778ef92c034891cac4d8e3f4f97, 0x01a155638c1972e394fe0a2168e48849, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 18:17:18.042332', '2026-10-19 18:19:09.722259', '2026-10-19 18:19:09.722259', 'rotated', '2026-10-19 18:17:18.042332'),
(0x01a155638c1972e394fe0a2168e48849, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0x57f2071e79219444a211d157c70503a03a2b17eacef7e29f9e3f2bd239c9d8fc, 0x01a156d2c1fa73918c0f96c12361b24a, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-18 18:19:09.721471', '2026-10-20 01:00:15.231856', '2026-10-20 01:00:15.231856', 'rotated', '2026-10-19 18:19:09.721471'),
(0x01a156d2c1fa73918c0f96c12361b24a, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0xc94b4ee0b0b3e5456f612743dbbfe4e6f15723a2291c03bc1ef7e7bda712cfa7, 0x01a156dc063673bc86bd26b8f9751fa5, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-19 01:00:15.228311', '2026-10-20 01:10:22.519806', '2026-10-20 01:10:22.519806', 'rotated', '2026-10-20 01:00:15.228311'),
(0x01a156dc063673bc86bd26b8f9751fa5, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0x0640ad0fee4708b788f5e91319bfab727d38e033f1184b85558d6ff992f97144, 0x01a156e1a8a3762192f3be00cab5077d, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-19 01:10:22.519218', '2026-10-20 01:16:32.032480', '2026-10-20 01:16:32.032480', 'rotated', '2026-10-20 01:10:22.519218'),
(0x01a156e1a8a3762192f3be00cab5077d, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0xb29b9217ce31a90b099c36224594ad606a768142aede8b73cda057cacefdb27b, 0x01a156e6ac8b7780a1b5dc426579a94e, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-19 01:16:31.870770', '2026-10-20 01:22:00.460844', '2026-10-20 01:22:00.460844', 'rotated', '2026-10-20 01:16:31.870770'),
(0x01a156e6ac8b7780a1b5dc426579a94e, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0x9274d402edd8997780ea920380ab6b1219a655187761a7692fbcdfb4693f5dda, 0x01a156e96be3727591a92a2b5145011a, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-19 01:22:00.460088', '2026-10-20 01:25:00.517089', '2026-10-20 01:25:00.517089', 'rotated', '2026-10-20 01:22:00.460088'),
(0x01a156e96be3727591a92a2b5145011a, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0xde02ec27ec9ac9cf86387a1ed5c9fad97f275d7d7b06e5537c1d15946bfcdac0, 0x01a156ec4261705cad806601b8c5a278, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-19 01:25:00.515731', '2026-10-20 01:28:06.498388', '2026-10-20 01:28:06.498388', 'rotated', '2026-10-20 01:25:00.515731'),
(0x01a156ec4261705cad806601b8c5a278, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0xdbbd8f049a23d87868a5326399139ea6c4042926e761fc206d76b96d06576580, 0x01a156f9889f77f695a63e5ad4023375, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-19 01:28:06.497661', '2026-10-20 01:42:36.449202', '2026-10-20 01:42:36.449202', 'rotated', '2026-10-20 01:28:06.497661'),
(0x01a156f9889f77f695a63e5ad4023375, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0x928cb777dbada6a37820ba022ae9403aa4c8e1f9ffcb2b263d006b2a4ed1d442, 0x01a15700b0c87150a40e28501401e7b8, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-19 01:42:36.448338', '2026-10-20 01:50:25.482930', '2026-10-20 01:50:25.482930', 'rotated', '2026-10-20 01:42:36.448338'),
(0x01a15700b0c87150a40e28501401e7b8, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0xac73eaa0b89a28c9a80eb48a384b603a654e6af0be2461aacd5b666340550b5a, 0x01a1570514f872b68a89c1c5c5bb90cf, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-19 01:50:25.481549', '2026-10-20 01:55:13.272609', '2026-10-20 01:55:13.272609', 'rotated', '2026-10-20 01:50:25.481549'),
(0x01a1570514f872b68a89c1c5c5bb90cf, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15204f9f7737d93cfe56cd183eb57, 0xcc7f1df233ba103609f1674b18a5dbb55b7026bff80084df440e3654bb116e32, NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:156.0) Gecko/20100101 Firefox/156.0', '2026-11-19 01:55:13.272217', '2026-10-20 01:55:13.272217', '2026-10-20 01:55:32.575580', 'reuse_detected', '2026-10-20 01:55:13.272217'),
(0x01a15a9e971074528e6c9e86e813e662, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9e971074528e6c9e8563ade834, 0x19fba64eb1d712d8f4412b1540157ade2b3defcce640930b5492dc548d9d8fa1, 0x01a17341c012775b9d312cde5f3f289e, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-19 18:41:45.232911', '2026-10-25 13:30:51.628019', '2026-10-25 13:30:51.628019', 'rotated', '2026-10-20 18:41:45.232911'),
(0x01a17341c012775b9d312cde5f3f289e, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9e971074528e6c9e8563ade834, 0xf0c313a6e84d2cd6cb589df2f287c7b3d5a4a0357a6dbe7b84f0e4f25a716f86, 0x01a17342e23e7718ae04bc97603472ec, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 13:30:51.626303', '2026-10-25 13:32:05.567765', '2026-10-25 13:32:05.567765', 'rotated', '2026-10-25 13:30:51.626303'),
(0x01a17342e23e7718ae04bc97603472ec, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9e971074528e6c9e8563ade834, 0xca278b0a148439c17ff7a6f76bbdee50caa184124f8d2a2e4c74699f6c21abb0, 0x01a1734305b077d89b23fa318a5730e3, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 13:32:05.566868', '2026-10-25 13:32:14.641909', '2026-10-25 13:32:14.641909', 'rotated', '2026-10-25 13:32:05.566868'),
(0x01a1734305b077d89b23fa318a5730e3, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a15a9e971074528e6c9e8563ade834, 0x2a0fcac4a93b6b60b276bcc4f520c55f57dcff5778b22062ad46e1997cea0a28, NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 13:32:14.641332', '2026-10-25 13:32:14.641332', '2026-10-25 13:32:23.441025', 'reuse_detected', '2026-10-25 13:32:14.641332'),
(0x01a173434783744188827fcb13ddf656, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0xf55c3d095c88a8244b858ff000f93e88ce46b9c4efe1cab55ef1b9fc478b7830, 0x01a173489332714a8a9194cd15f70f87, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 13:32:31.493691', '2026-10-25 13:38:18.547776', '2026-10-25 13:38:18.547776', 'rotated', '2026-10-25 13:32:31.493691'),
(0x01a173489332714a8a9194cd15f70f87, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x27e6d6de92fbb260aefad0d832e2e77d7597fd3925c562bf64fe9e105654e82f, 0x01a17348a72d71d39ef51e6b4df41068, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 13:38:18.546785', '2026-10-25 13:38:23.662575', '2026-10-25 13:38:23.662575', 'rotated', '2026-10-25 13:38:18.546785'),
(0x01a17348a72d71d39ef51e6b4df41068, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x82668539d604128f51ab598ccf43d049c7c27b8dbf3f840e0622c187ba0f82dc, 0x01a1734b36a875ebbf23c5e1a3105443, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 13:38:23.661755', '2026-10-25 13:41:11.466058', '2026-10-25 13:41:11.466058', 'rotated', '2026-10-25 13:38:23.661755'),
(0x01a1734b36a875ebbf23c5e1a3105443, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x61b7895481a3f508d15366b4b21ba8ccc9572b6d7fbc14e187acf9630e6d3f9d, 0x01a1734c200b7134b4264a36883c903c, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 13:41:11.464994', '2026-10-25 13:42:11.212817', '2026-10-25 13:42:11.212817', 'rotated', '2026-10-25 13:41:11.464994'),
(0x01a1734c200b7134b4264a36883c903c, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0xec5fc2200b6ea0cc05c542be66118859a7760931683d30d75e2dd9ada5aed416, 0x01a17353a6f57461b553afafa488c4df, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 13:42:11.212170', '2026-10-25 13:50:24.504872', '2026-10-25 13:50:24.504872', 'rotated', '2026-10-25 13:42:11.212170'),
(0x01a17353a6f57461b553afafa488c4df, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x4e7062f92c3cdaf3c3d4b5d388779b79799347dbedae4ac0fa04d855b31bdb79, 0x01a173576d9e7007b47f79610c690c48, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 13:50:24.503047', '2026-10-25 13:54:31.967020', '2026-10-25 13:54:31.967020', 'rotated', '2026-10-25 13:50:24.503047'),
(0x01a173576d9e7007b47f79610c690c48, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x286bf8dcb9dc593c7a6306b16f298ba7218fcc4a71b4cbe7061c1641bf9648d5, 0x01a1736e625c71ab962f362e06a2a813, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 13:54:31.966693', '2026-10-25 14:19:36.606144', '2026-10-25 14:19:36.606144', 'rotated', '2026-10-25 13:54:31.966693'),
(0x01a1736e625c71ab962f362e06a2a813, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x15e538bae8eceb649b74eb4d3c7979e974e833b039389abfb519cba8f39993a0, 0x01a17370bfa37271b85b1869ac6fb955, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 14:19:36.412912', '2026-10-25 14:22:11.364643', '2026-10-25 14:22:11.364643', 'rotated', '2026-10-25 14:19:36.412912'),
(0x01a17370bfa37271b85b1869ac6fb955, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x2c4b100b2d1ee5b98cc3aa08bfe7bac92883fbfe7afb3f1f383f912992224ff3, 0x01a1737bf6c7774aa06879438f436671, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 14:22:11.364042', '2026-10-25 14:34:26.376861', '2026-10-25 14:34:26.376861', 'rotated', '2026-10-25 14:22:11.364042'),
(0x01a1737bf6c7774aa06879438f436671, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x38d2217d8bacbb6f319a1f0672c6d8409bd20e56f744b51be18704756ad54b62, 0x01a1737f9b2f754f8569c23dc264d2a8, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 14:34:26.376061', '2026-10-25 14:38:25.072350', '2026-10-25 14:38:25.072350', 'rotated', '2026-10-25 14:34:26.376061'),
(0x01a1737f9b2f754f8569c23dc264d2a8, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0xe91c5238a907846e0d1731af7b70a88fc6f530174091ceb81e27355045071817, 0x01a173856afa746e87283111437d744f, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 14:38:25.071579', '2026-10-25 14:44:45.947053', '2026-10-25 14:44:45.947053', 'rotated', '2026-10-25 14:38:25.071579'),
(0x01a173856afa746e87283111437d744f, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x023a99cfe13bd23e675a8707b702546a4a886406d3744f82698a76c48247a5d1, 0x01a173857e0170369fd32b7a9960af3a, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 14:44:45.946386', '2026-10-25 14:44:50.818813', '2026-10-25 14:44:50.818813', 'rotated', '2026-10-25 14:44:45.946386'),
(0x01a173857e0170369fd32b7a9960af3a, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0xfc182fda0dbde288139bf8dc6fa3efb0f4fc73230f2c8124c1521cdc24d88fc8, 0x01a1738a2333774c9060898d7124ac8f, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 14:44:50.818090', '2026-10-25 14:49:55.253624', '2026-10-25 14:49:55.253624', 'rotated', '2026-10-25 14:44:50.818090'),
(0x01a1738a2333774c9060898d7124ac8f, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x11655b8dee9845032f24e7dff860fa39626c48bb868f05a5e823e260b5fd1ff9, 0x01a173bc58237655b84b9a42cf216544, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 14:49:55.252554', '2026-10-25 15:44:45.606586', '2026-10-25 15:44:45.606586', 'rotated', '2026-10-25 14:49:55.252554');
INSERT INTO `user_sessions` (`id`, `user_id`, `tenant_id`, `family_id`, `refresh_token_hash`, `replaced_by_id`, `ip_address`, `user_agent`, `expires_at`, `last_used_at`, `revoked_at`, `revoked_reason`, `created_at`) VALUES
(0x01a173bc58237655b84b9a42cf216544, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x316560cd19c672aa22e95c968e69705bde5cd0ce5851fd58d8b092c4e8b40e6e, 0x01a173bda1667465bc596a3e9d3eef20, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 15:44:45.604635', '2026-10-25 15:46:09.895061', '2026-10-25 15:46:09.895061', 'rotated', '2026-10-25 15:44:45.604635'),
(0x01a173bda1667465bc596a3e9d3eef20, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x0b3d07ae4b148adf3a94f62b4d2c17d964737d3e82b40371d79c73f03cfd7f17, 0x01a173c0bc03707e9132f6c35aa65986, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 15:46:09.894743', '2026-10-25 15:49:33.316882', '2026-10-25 15:49:33.316882', 'rotated', '2026-10-25 15:46:09.894743'),
(0x01a173c0bc03707e9132f6c35aa65986, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x104429fe57bfb5afbf133ecc771017efd63789e80dd44a2e79d3755c191161be, 0x01a173c1c74a75e8982f39b3de592821, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 15:49:33.316177', '2026-10-25 15:50:41.739965', '2026-10-25 15:50:41.739965', 'rotated', '2026-10-25 15:49:33.316177'),
(0x01a173c1c74a75e8982f39b3de592821, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x11298878d260643a7da405de5eb272236bd460c41a53ff6b84110786891041c2, 0x01a173c2835f70b4864d40ae9666ff3e, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 15:50:41.739043', '2026-10-25 15:51:29.888156', '2026-10-25 15:51:29.888156', 'rotated', '2026-10-25 15:50:41.739043'),
(0x01a173c2835f70b4864d40ae9666ff3e, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173434783744188827fcacab90ed9, 0x3e1d8ca187132bd63bdf4c720e3b3a95e815dc68e5cdce5e204f54271ad9c5a0, NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 15:51:29.887461', '2026-10-25 15:51:29.887461', '2026-10-25 15:51:30.583789', 'reuse_detected', '2026-10-25 15:51:29.887461'),
(0x01a173c2bd74737d948208668d0a2f11, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0x2777f1a7d2477f348a73b325e07a7f23c97174782a093c04af50435d632a92a2, 0x01a173c71b5177f38ac82316ae2f78fa, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 15:51:44.757054', '2026-10-25 15:56:30.930501', '2026-10-25 15:56:30.930501', 'rotated', '2026-10-25 15:51:44.757054'),
(0x01a173c71b5177f38ac82316ae2f78fa, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0x1b8c9176fe8a4ff728081c1a0bb75d143bb3a49e1dd2c5628732fa7294356594, 0x01a173cabb07743c8309880d73576ab8, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 15:56:30.930105', '2026-10-25 16:00:28.425497', '2026-10-25 16:00:28.425497', 'rotated', '2026-10-25 15:56:30.930105'),
(0x01a173cabb07743c8309880d73576ab8, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xd84eb92c4c7ca339b8b575278c9bb160a6728346f055bc5d9ee273ddf3e90706, 0x01a173cb008d778299755bf2849b9e47, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:00:28.424470', '2026-10-25 16:00:46.222624', '2026-10-25 16:00:46.222624', 'rotated', '2026-10-25 16:00:28.424470'),
(0x01a173cb008d778299755bf2849b9e47, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xf5f3722355496070dd5c70e129ebb44be973cb1578a1e1323b9a1eefd246f9a9, 0x01a173d0221d74e987966faa885b0027, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:00:46.222133', '2026-10-25 16:06:22.493846', '2026-10-25 16:06:22.493846', 'rotated', '2026-10-25 16:00:46.222133'),
(0x01a173d0221d74e987966faa885b0027, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xf363b63c63e04bc58a322f5b8f0211ebb009aaea654016ac2b291cfd4dd85d7c, 0x01a173d4105475b489b29d12695e9092, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:06:22.493225', '2026-10-25 16:10:40.085080', '2026-10-25 16:10:40.085080', 'rotated', '2026-10-25 16:06:22.493225'),
(0x01a173d4105475b489b29d12695e9092, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0x3bde413cba623bf21016ba073ea1025a549cc7ce575ead14443bdf30e3cd8e27, 0x01a173d4fb3d75518766d59c522a4a0c, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:10:40.084746', '2026-10-25 16:11:40.222586', '2026-10-25 16:11:40.222586', 'rotated', '2026-10-25 16:10:40.084746'),
(0x01a173d4fb3d75518766d59c522a4a0c, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xba59138f9672e166647e5738f1e7f5f933fe61b716822bba00579c71ef280064, 0x01a173d90dd1750bb0f068d646fdfa5a, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:11:40.222010', '2026-10-25 16:16:07.123051', '2026-10-25 16:16:07.123051', 'rotated', '2026-10-25 16:11:40.222010'),
(0x01a173d90dd1750bb0f068d646fdfa5a, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0x72b1f560761c31317999ff7da5586459f7110a2b9d342467c64695fb690afa59, 0x01a173db42167376a841a13a6923a058, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:16:07.122299', '2026-10-25 16:18:31.575490', '2026-10-25 16:18:31.575490', 'rotated', '2026-10-25 16:16:07.122299'),
(0x01a173db42167376a841a13a6923a058, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xf3036e12f63f4829a2f97dc3cf7432547ab961311341e1366b70f044ae61c56d, 0x01a173dd881171cdbd46ab34aec7160e, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:18:31.574552', '2026-10-25 16:21:00.562025', '2026-10-25 16:21:00.562025', 'rotated', '2026-10-25 16:18:31.574552'),
(0x01a173dd881171cdbd46ab34aec7160e, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xdc237ae150722cd8899b48615551f2a4ec9cfc7ea13e434f439282f8e6abe12a, 0x01a173ec1ecb7174990c0429d3ec3710, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:21:00.561418', '2026-10-25 16:36:56.656880', '2026-10-25 16:36:56.656880', 'rotated', '2026-10-25 16:21:00.561418'),
(0x01a173ec1ecb7174990c0429d3ec3710, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0x6c1102089bd2c6e2c054f6fc393143321dd00f9fba841eff7790e2d03b60df3b, 0x01a173f08da6730cb5e0a4ea6cd6c00b, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:36:56.653615', '2026-10-25 16:41:47.175134', '2026-10-25 16:41:47.175134', 'rotated', '2026-10-25 16:36:56.653615'),
(0x01a173f08da6730cb5e0a4ea6cd6c00b, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xc5748fc0e9de9f94334ece8cefd9596e069e4a15ffc4f262bc35ebf36f813388, 0x01a173ff46f0743a93f468a867a8068b, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:41:47.174485', '2026-10-25 16:57:52.114036', '2026-10-25 16:57:52.114036', 'rotated', '2026-10-25 16:41:47.174485'),
(0x01a173ff46f0743a93f468a867a8068b, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xe28347d0e5057988b27ea49179259349bede477443e0c3e41db3fcf3d00bc2f5, 0x01a1740023737286817b4072d994690a, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:57:52.112826', '2026-10-25 16:58:48.564443', '2026-10-25 16:58:48.564443', 'rotated', '2026-10-25 16:57:52.112826'),
(0x01a1740023737286817b4072d994690a, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xd66748b56e144405ce8ba2c5f0cc0ecc0018e6fee48fde48329de24113dd840f, 0x01a174ee0d3f7681b5cf60bcf9ac9335, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 16:58:48.563671', '2026-10-25 21:18:40.448840', '2026-10-25 21:18:40.448840', 'rotated', '2026-10-25 16:58:48.563671'),
(0x01a174ee0d3f7681b5cf60bcf9ac9335, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0x971ae859452477466ff9effd0b121eadc9ec499bc74083aaca2da85194da37b2, 0x01a174ee5255710dab1c784d5b527cc3, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 21:18:40.448192', '2026-10-25 21:18:58.135036', '2026-10-25 21:18:58.135036', 'rotated', '2026-10-25 21:18:40.448192'),
(0x01a174ee5255710dab1c784d5b527cc3, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0x87353b59198c24aeb28ed4acea299cd62f6e4d54031d7ca3100ceb1ec66e3d29, 0x01a174f42d4b77e1982e5b7372c6204f, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 21:18:58.134274', '2026-10-25 21:25:21.868491', '2026-10-25 21:25:21.868491', 'rotated', '2026-10-25 21:18:58.134274'),
(0x01a174f42d4b77e1982e5b7372c6204f, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0x89a06004c256211e2d441edb32179bbee4b6920fef3060a5c52785d6f6b9056a, 0x01a174f5368170a98e215b3edc783620, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 21:25:21.867811', '2026-10-25 21:26:29.763541', '2026-10-25 21:26:29.763541', 'rotated', '2026-10-25 21:25:21.867811'),
(0x01a174f5368170a98e215b3edc783620, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xecc6a5d28c50fd92bf0276330d62d7421479eea8afaba6e04afc35ffbd32e26c, 0x01a1755f8bad727aba4be715d4e7685a, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 21:26:29.762303', '2026-10-25 23:22:38.383874', '2026-10-25 23:22:38.383874', 'rotated', '2026-10-25 21:26:29.762303'),
(0x01a1755f8bad727aba4be715d4e7685a, 0x01a14c8ff09475ea92a85fc5af151c9e, 0x01a14c8ff2df764db434b8b65d8bac97, 0x01a173c2bd74737d948208651fdc4a45, 0xb34153a82bb1631635fa2c62acda3f5a07dd3845bc9c68c6228d554d70fcfcf9, NULL, 0x7f000001, 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:158.0) Gecko/20100101 Firefox/158.0', '2026-11-24 23:22:38.382713', '2026-10-25 23:22:38.382713', NULL, NULL, '2026-10-25 23:22:38.382713');

-- --------------------------------------------------------

--
-- Table structure for table `_sqlx_migrations`
--

CREATE TABLE `_sqlx_migrations` (
  `version` bigint NOT NULL,
  `description` text NOT NULL,
  `installed_on` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `success` tinyint(1) NOT NULL,
  `checksum` blob NOT NULL,
  `execution_time` bigint NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

--
-- Dumping data for table `_sqlx_migrations`
--

INSERT INTO `_sqlx_migrations` (`version`, `description`, `installed_on`, `success`, `checksum`, `execution_time`) VALUES
(1, 'setup', '2026-10-18 01:05:34', 1, 0xb768fae73f551fd099b92857e0e5324b6ff09c784af0f916bd3623e0855461fcc34cf994d2898800f98580f5a356dd74, 309620834),
(2, 'platform core', '2026-10-18 01:05:35', 1, 0x1dbe243dfdcc68df87c2ad4bad04383c8ba6d615abfaf0527999e6a15678e44f96557869d5b7d0dfd8be198f122226ca, 3859868674),
(3, 'identity', '2026-10-18 01:05:39', 1, 0xa7a2ade52fc6c5274acf8badded3ea6bf5e3b890e8e52d3154f4f6715883d3ddd3164045ef1981e00702a330b6e60dc8, 2549823855),
(4, 'tenancy', '2026-10-18 01:05:41', 1, 0x167129c908977538d3099efc4e3aa6d475d91ec57af1fc84fd5c5055918c90e9c8701830e0e93f45635a81d4f30ea41a, 6498020406),
(5, 'rbac', '2026-10-18 01:05:48', 1, 0x5f45afd664e742bc2cf0cd371916c995c6fe7546b5a30d08a9d28b621f965489d26d62ec3166b08f5058b4fa3159d550, 3273613881),
(6, 'billing core', '2026-10-18 01:05:51', 1, 0xa0902cc7b4adcfc56f0f01cbc084f3bc2c53d262186748ef98fcc9355066ac9f32c1ee638c8113d3efc1183f1f582b04, 4535361022),
(7, 'audit idempotency', '2026-10-18 01:05:56', 1, 0x59aa499a7d276abf4d3b91428b7c113289021833d376b77aea8cd5b8c1cc712c44b85f8807845175fc66312c3e906e8a, 4577878074),
(8, 'auth lockout index', '2026-10-18 01:06:01', 1, 0x7814dd2486a2ac75e00b770af41134f2951d391bf72a875e56ea872cde0eaf7f9d9aaf4fe88a0d85de0496c143ded95e, 559126006),
(9, 'catalog', '2026-10-18 01:06:01', 1, 0xd0144288ef61e76f5b06e043c4c6c228c3db838d5fefaf71427ba333e4b8dc555bc56af4d79cbc986e6afa1e612c8bf9, 11992404973),
(10, 'inventory', '2026-10-18 06:05:21', 1, 0x1bf5cd31dcb50f5f956597aea58fbbc31d1715b7532fad1cd5989244ae3283bdeaf6e7ca78edf70c44d1f82ed48fbbf4, 6679617317),
(12, 'money', '2026-10-18 06:05:28', 1, 0xea2b28c69dd366d85ac57eaa9db65362c46f058ff44f24d2d799c8bd9598b389c89fde79e003d80049b0817cb4407612, 6152055895),
(13, 'parties', '2026-10-18 06:05:34', 1, 0x2c458a45ce182af3269719314a9ba9171c989da1b5725a49069b5781d4b46690a6a0d4035fbbe497a8c4c1f6f3a0ea3c, 3100442751),
(14, 'sales', '2026-10-18 06:05:37', 1, 0x1e5e4d734899ffcfc744f9838f717f99571a338067e925fba3e55056d3505e810f3898c163de85b7eccf161c39e00526, 3155462837),
(15, 'returns', '2026-10-18 06:05:40', 1, 0x0bf03eba2c30be656ce8a9b22d9e7e412e4e9da1c348690c15dcfcca6d476a97c0cb2d3e0fc7510295c2eaf50a25a0b8, 2376173985),
(16, 'purchasing', '2026-10-18 06:05:43', 1, 0x49263a2083d7197c0bd3ef2d423e1afddbf5b99e279197b4c35cf59e9752d584056683901b945d8b6bcdb1a31c369915, 4231185683),
(17, 'expenses', '2026-10-18 06:05:47', 1, 0x2a1f8a89c3bdd79462a19687b3172169d79b5a84716daacfad1333d2ba0f4f372dc9586a0860f6590fe298206e598202, 2210842943),
(19, 'platform admin', '2026-10-19 02:06:49', 1, 0x83c0ee52b23abb36cd32f910795b42747d0bc0d35924075067a9d0521f14efd45f95bb61797258b359ff52416c32973b, 45552393252),
(101, 'r2 inventory', '2026-10-19 18:15:14', 1, 0x6ffd5c3a811bf316ac90135549bcc198cdc07671f62c69b741790ea15c1b38416a4117b92244e5bcc5e2f53be07992f2, 9388944922),
(103, 'r2 purchasing', '2026-10-18 06:05:49', 1, 0x1eb8cfce0ce67cfc073551e40f3645c09be4b3707d44834ef1ac7621fed33fc91ef0b4bec9a6066482bb924492bec5d7, 2991442725),
(104, 'r2 pos', '2026-10-19 18:15:24', 1, 0xb29720fb86e2c04e3f43b6cabd7755ac72f97aeaacb8feed8a68227e176160bf49d48591171ceb1903e65951b0b610e0, 12508996712),
(105, 'r2 accounting', '2026-10-18 06:05:53', 1, 0x4103f34bbe4e9bdcba107317b2e36ad900f7e1d6b15eb345f86326cf7894087a908a59b58c531263d6c84626c9966823, 8669256796),
(106, 'r2 money', '2026-10-19 18:15:37', 1, 0x080d289fca345f593ff2f7f3f6d61c45dde8dad5edd67dd9e108f839f15d9c7d660443fbfb79b728964d5b680b0b21af, 5469259366),
(107, 'r2 purchase orders', '2026-10-19 18:15:43', 1, 0xdde5cbf9a01ffc0854fef572889a1fbb684ddb8d5edd72a1e0f81636ea222ec5beeabee8adf34175a99fd0bd09ae90d0, 5008165154),
(108, 'r2 bank reconciliation', '2026-10-19 18:15:48', 1, 0x899302282b85a1c883bb59d45eb1e08db8730d0ed2cf302718c7150b03419e8ea593de433c459534ea91700b46f53b83, 2250176874),
(109, 'r2 notifications', '2026-10-20 18:41:02', 1, 0x2e7a9079291a96d715e70c52eee24dbb3a94c99ddbf6fb2df2e9d7037b6efb89a5f9b5644247c293ebf598305fee6125, 3213349602),
(110, 'lifetime licence', '2026-10-25 14:36:01', 1, 0xcec010c74ddf813371a43ea1b586a7f50d11694caf5a38996a4ae009f31a6e3ee7800905905f513d6dff6c4a4140fe52, 15262115022),
(111, 'files', '2026-10-25 15:43:45', 1, 0xd4155354bbd5f965159824189951189f933910ac8aa0825a41d160f99d38e6d318fb59a38e9b988affc267e6235e751a, 15382522049);

--
-- Indexes for dumped tables
--

--
-- Indexes for table `account_transactions`
--
ALTER TABLE `account_transactions`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_account_txn_book` (`tenant_id`,`account_id`,`business_date`,`created_at`),
  ADD KEY `idx_account_txn_ref` (`tenant_id`,`reference_type`,`reference_id`);

--
-- Indexes for table `account_transfers`
--
ALTER TABLE `account_transfers`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_account_transfers_no` (`tenant_id`,`transfer_no`),
  ADD KEY `idx_account_transfers_date` (`tenant_id`,`business_date`),
  ADD KEY `fk_acc_transfers_branch` (`tenant_id`,`branch_id`),
  ADD KEY `fk_acc_transfers_from` (`tenant_id`,`from_account_id`),
  ADD KEY `fk_acc_transfers_to` (`tenant_id`,`to_account_id`);

--
-- Indexes for table `audit_logs`
--
ALTER TABLE `audit_logs`
  ADD PRIMARY KEY (`id`,`created_at`),
  ADD KEY `idx_audit_tenant_time` (`tenant_id`,`created_at`),
  ADD KEY `idx_audit_entity` (`tenant_id`,`entity_type`,`entity_id`),
  ADD KEY `idx_audit_actor` (`tenant_id`,`actor_id`,`created_at`);

--
-- Indexes for table `auth_events`
--
ALTER TABLE `auth_events`
  ADD PRIMARY KEY (`id`),
  ADD KEY `idx_auth_events_user` (`user_id`,`created_at`),
  ADD KEY `idx_auth_events_tenant` (`tenant_id`,`created_at`),
  ADD KEY `idx_auth_events_identifier` (`identifier`,`event`,`created_at`);

--
-- Indexes for table `bank_reconciliations`
--
ALTER TABLE `bank_reconciliations`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_bank_rec_open` (`tenant_id`,`open_account_id`),
  ADD KEY `idx_bank_rec_account` (`tenant_id`,`account_id`,`statement_date`);

--
-- Indexes for table `bank_reconciliation_items`
--
ALTER TABLE `bank_reconciliation_items`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_bank_rec_items_txn` (`tenant_id`,`account_transaction_id`),
  ADD KEY `idx_bank_rec_items_rec` (`tenant_id`,`reconciliation_id`);

--
-- Indexes for table `branches`
--
ALTER TABLE `branches`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_branches_code` (`tenant_id`,`code`),
  ADD UNIQUE KEY `uq_branches_default` (`tenant_id`,`default_key`),
  ADD KEY `fk_branches_default_location` (`tenant_id`,`default_location_id`);

--
-- Indexes for table `brands`
--
ALTER TABLE `brands`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_brands_name` (`tenant_id`,`live_name`);

--
-- Indexes for table `cash_registers`
--
ALTER TABLE `cash_registers`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_cash_registers_name` (`tenant_id`,`branch_id`,`name`),
  ADD KEY `fk_registers_account` (`tenant_id`,`cash_account_id`);

--
-- Indexes for table `categories`
--
ALTER TABLE `categories`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_categories_name` (`tenant_id`,`parent_key`,`live_name`),
  ADD KEY `fk_categories_parent` (`tenant_id`,`parent_id`);

--
-- Indexes for table `customers`
--
ALTER TABLE `customers`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_customers_code` (`tenant_id`,`code`),
  ADD UNIQUE KEY `uq_customers_phone` (`tenant_id`,`live_phone`),
  ADD UNIQUE KEY `uq_customers_walk_in` (`tenant_id`,`walk_in_key`),
  ADD KEY `idx_customers_name` (`tenant_id`,`name`),
  ADD KEY `idx_customers_due` (`tenant_id`,`current_balance`);

--
-- Indexes for table `document_sequences`
--
ALTER TABLE `document_sequences`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_document_sequences` (`tenant_id`,`branch_key`,`doc_type`),
  ADD KEY `fk_docseq_branch` (`tenant_id`,`branch_id`);

--
-- Indexes for table `expenses`
--
ALTER TABLE `expenses`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_expenses_no` (`tenant_id`,`branch_id`,`expense_no`),
  ADD UNIQUE KEY `uq_expenses_idem` (`tenant_id`,`idempotency_key`),
  ADD KEY `idx_expenses_date` (`tenant_id`,`branch_id`,`business_date`),
  ADD KEY `idx_expenses_category` (`tenant_id`,`category_id`,`business_date`),
  ADD KEY `fk_expenses_account` (`tenant_id`,`account_id`),
  ADD KEY `fk_expenses_payment` (`tenant_id`,`payment_id`);

--
-- Indexes for table `expense_categories`
--
ALTER TABLE `expense_categories`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_expense_categories_name` (`tenant_id`,`name`),
  ADD KEY `fk_expense_categories_ledger` (`tenant_id`,`ledger_account_id`);

--
-- Indexes for table `features`
--
ALTER TABLE `features`
  ADD PRIMARY KEY (`code`);

--
-- Indexes for table `files`
--
ALTER TABLE `files`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_files_storage_key` (`storage_key`),
  ADD KEY `idx_files_entity` (`tenant_id`,`entity_type`,`entity_id`);

--
-- Indexes for table `fiscal_periods`
--
ALTER TABLE `fiscal_periods`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_fiscal_periods_start` (`tenant_id`,`start_date`);

--
-- Indexes for table `goods_receipts`
--
ALTER TABLE `goods_receipts`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_goods_receipts_no` (`tenant_id`,`receipt_no`),
  ADD KEY `idx_grn_purchase` (`tenant_id`,`purchase_id`),
  ADD KEY `fk_grn_location` (`tenant_id`,`location_id`);

--
-- Indexes for table `goods_receipt_items`
--
ALTER TABLE `goods_receipt_items`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `fk_grn_items_receipt` (`tenant_id`,`receipt_id`),
  ADD KEY `fk_grn_items_pi` (`tenant_id`,`purchase_item_id`),
  ADD KEY `fk_grn_items_variant` (`tenant_id`,`variant_id`);

--
-- Indexes for table `held_carts`
--
ALTER TABLE `held_carts`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_held_carts_branch` (`tenant_id`,`branch_id`,`created_at`);

--
-- Indexes for table `idempotency_keys`
--
ALTER TABLE `idempotency_keys`
  ADD PRIMARY KEY (`tenant_id`,`idem_key`),
  ADD KEY `idx_idempotency_expiry` (`expires_at`);

--
-- Indexes for table `income_categories`
--
ALTER TABLE `income_categories`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_income_categories_name` (`tenant_id`,`name`),
  ADD KEY `fk_income_categories_ledger` (`tenant_id`,`ledger_account_id`);

--
-- Indexes for table `invitations`
--
ALTER TABLE `invitations`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_invitations_token` (`token_hash`),
  ADD KEY `fk_invitations_user` (`accepted_by`);

--
-- Indexes for table `journal_entries`
--
ALTER TABLE `journal_entries`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_journal_entries_no` (`tenant_id`,`entry_no`),
  ADD KEY `idx_journal_source` (`tenant_id`,`source_type`,`source_id`),
  ADD KEY `idx_journal_date` (`tenant_id`,`entry_date`),
  ADD KEY `fk_journal_reversal` (`tenant_id`,`reversal_of_id`);

--
-- Indexes for table `journal_lines`
--
ALTER TABLE `journal_lines`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_journal_lines_account` (`tenant_id`,`account_id`,`entry_date`),
  ADD KEY `idx_journal_lines_entry` (`tenant_id`,`entry_id`);

--
-- Indexes for table `ledger_accounts`
--
ALTER TABLE `ledger_accounts`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_ledger_code` (`tenant_id`,`code`),
  ADD UNIQUE KEY `uq_ledger_system_key` (`tenant_id`,`system_key`),
  ADD KEY `fk_ledger_parent` (`tenant_id`,`parent_id`);

--
-- Indexes for table `membership_roles`
--
ALTER TABLE `membership_roles`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_membership_roles` (`tenant_id`,`membership_id`,`role_id`,`branch_key`),
  ADD KEY `fk_mr_role` (`tenant_id`,`role_id`),
  ADD KEY `fk_mr_branch` (`tenant_id`,`branch_id`);

--
-- Indexes for table `notifications`
--
ALTER TABLE `notifications`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_notifications_dedupe` (`tenant_id`,`user_id`,`dedupe_key`),
  ADD KEY `idx_notifications_user` (`tenant_id`,`user_id`,`read_at`,`created_at`),
  ADD KEY `idx_notifications_dedupe` (`tenant_id`,`dedupe_key`),
  ADD KEY `fk_notifications_user` (`user_id`);

--
-- Indexes for table `notification_deliveries`
--
ALTER TABLE `notification_deliveries`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_notif_deliveries_due` (`status`,`next_attempt_at`),
  ADD KEY `idx_notif_deliveries_notification` (`tenant_id`,`notification_id`);

--
-- Indexes for table `notification_preferences`
--
ALTER TABLE `notification_preferences`
  ADD PRIMARY KEY (`tenant_id`,`user_id`,`notif_type`),
  ADD KEY `fk_notif_prefs_user` (`user_id`);

--
-- Indexes for table `other_incomes`
--
ALTER TABLE `other_incomes`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_other_incomes_no` (`tenant_id`,`branch_id`,`income_no`),
  ADD KEY `idx_other_incomes_date` (`tenant_id`,`business_date`),
  ADD KEY `fk_other_incomes_category` (`tenant_id`,`category_id`),
  ADD KEY `fk_other_incomes_account` (`tenant_id`,`account_id`);

--
-- Indexes for table `otp_codes`
--
ALTER TABLE `otp_codes`
  ADD PRIMARY KEY (`id`),
  ADD KEY `idx_otp_destination` (`destination`,`purpose`,`created_at`),
  ADD KEY `fk_otp_user` (`user_id`);

--
-- Indexes for table `outbox_events`
--
ALTER TABLE `outbox_events`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_outbox_unpublished` (`published_at`,`created_at`);

--
-- Indexes for table `party_ledger_entries`
--
ALTER TABLE `party_ledger_entries`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_party_ledger_party` (`tenant_id`,`party_type`,`party_id`,`business_date`,`created_at`),
  ADD KEY `idx_party_ledger_ref` (`tenant_id`,`reference_type`,`reference_id`);

--
-- Indexes for table `payments`
--
ALTER TABLE `payments`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_payments_no` (`tenant_id`,`branch_id`,`payment_no`),
  ADD KEY `idx_payments_party` (`tenant_id`,`party_type`,`party_id`,`payment_date`),
  ADD KEY `idx_payments_date` (`tenant_id`,`branch_id`,`business_date`),
  ADD KEY `idx_payments_account` (`tenant_id`,`account_id`,`business_date`),
  ADD KEY `fk_payments_method` (`tenant_id`,`payment_method_id`),
  ADD KEY `idx_payments_shift` (`tenant_id`,`register_shift_id`);

--
-- Indexes for table `payment_accounts`
--
ALTER TABLE `payment_accounts`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_payment_accounts_name` (`tenant_id`,`name`),
  ADD KEY `fk_payment_accounts_branch` (`tenant_id`,`branch_id`),
  ADD KEY `fk_payment_accounts_ledger` (`tenant_id`,`ledger_account_id`);

--
-- Indexes for table `payment_allocations`
--
ALTER TABLE `payment_allocations`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_allocations_document` (`tenant_id`,`document_type`,`document_id`),
  ADD KEY `idx_allocations_payment` (`tenant_id`,`payment_id`);

--
-- Indexes for table `payment_methods`
--
ALTER TABLE `payment_methods`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_payment_methods_name` (`tenant_id`,`name`),
  ADD KEY `fk_payment_methods_account` (`tenant_id`,`default_account_id`);

--
-- Indexes for table `permissions`
--
ALTER TABLE `permissions`
  ADD PRIMARY KEY (`code`);

--
-- Indexes for table `plans`
--
ALTER TABLE `plans`
  ADD PRIMARY KEY (`id`),
  ADD UNIQUE KEY `uq_plans_code` (`code`);

--
-- Indexes for table `plan_features`
--
ALTER TABLE `plan_features`
  ADD PRIMARY KEY (`plan_id`,`feature_code`),
  ADD KEY `fk_plan_features_feature` (`feature_code`);

--
-- Indexes for table `plan_prices`
--
ALTER TABLE `plan_prices`
  ADD PRIMARY KEY (`id`),
  ADD UNIQUE KEY `uq_plan_prices` (`plan_id`,`billing_cycle`,`currency_code`);

--
-- Indexes for table `platform_sequences`
--
ALTER TABLE `platform_sequences`
  ADD PRIMARY KEY (`name`);

--
-- Indexes for table `platform_settings`
--
ALTER TABLE `platform_settings`
  ADD PRIMARY KEY (`setting_key`);

--
-- Indexes for table `platform_staff`
--
ALTER TABLE `platform_staff`
  ADD PRIMARY KEY (`user_id`);

--
-- Indexes for table `products`
--
ALTER TABLE `products`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_products_category` (`tenant_id`,`category_id`),
  ADD KEY `idx_products_name` (`tenant_id`,`name`),
  ADD KEY `fk_products_brand` (`tenant_id`,`brand_id`),
  ADD KEY `fk_products_unit` (`tenant_id`,`base_unit_id`),
  ADD KEY `fk_products_tax` (`tenant_id`,`tax_rate_id`),
  ADD KEY `fk_products_image` (`tenant_id`,`image_file_id`);

--
-- Indexes for table `product_barcodes`
--
ALTER TABLE `product_barcodes`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_barcodes` (`tenant_id`,`barcode`),
  ADD KEY `idx_barcodes_variant` (`tenant_id`,`variant_id`),
  ADD KEY `fk_barcodes_unit` (`tenant_id`,`unit_id`);

--
-- Indexes for table `product_price_history`
--
ALTER TABLE `product_price_history`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_price_history_variant` (`tenant_id`,`variant_id`,`changed_at`);

--
-- Indexes for table `product_variants`
--
ALTER TABLE `product_variants`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_variants_sku` (`tenant_id`,`live_sku`),
  ADD UNIQUE KEY `uq_variants_default` (`tenant_id`,`product_id`,`default_key`),
  ADD KEY `idx_variants_product` (`tenant_id`,`product_id`),
  ADD KEY `idx_variants_search_prefix` (`tenant_id`,`search_text`(64));
ALTER TABLE `product_variants` ADD FULLTEXT KEY `ft_variants_search` (`search_text`);

--
-- Indexes for table `purchases`
--
ALTER TABLE `purchases`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_purchases_no` (`tenant_id`,`branch_id`,`purchase_no`),
  ADD UNIQUE KEY `uq_purchases_idem` (`tenant_id`,`idempotency_key`),
  ADD UNIQUE KEY `uq_purchases_supplier_invoice` (`tenant_id`,`supplier_id`,`live_supplier_invoice`),
  ADD KEY `idx_purchases_supplier` (`tenant_id`,`supplier_id`,`purchase_date`),
  ADD KEY `idx_purchases_date` (`tenant_id`,`branch_id`,`purchase_date`),
  ADD KEY `idx_purchases_due` (`tenant_id`,`supplier_id`,`due_total`),
  ADD KEY `fk_purchases_location` (`tenant_id`,`location_id`);

--
-- Indexes for table `purchase_items`
--
ALTER TABLE `purchase_items`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_purchase_items_line` (`tenant_id`,`purchase_id`,`line_no`),
  ADD KEY `idx_purchase_items_variant` (`tenant_id`,`variant_id`);

--
-- Indexes for table `purchase_returns`
--
ALTER TABLE `purchase_returns`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_purchase_returns_no` (`tenant_id`,`branch_id`,`return_no`),
  ADD UNIQUE KEY `uq_purchase_returns_idem` (`tenant_id`,`idempotency_key`),
  ADD KEY `idx_purchase_returns_purchase` (`tenant_id`,`purchase_id`),
  ADD KEY `idx_purchase_returns_date` (`tenant_id`,`business_date`),
  ADD KEY `fk_pr_supplier` (`tenant_id`,`supplier_id`),
  ADD KEY `fk_pr_location` (`tenant_id`,`location_id`);

--
-- Indexes for table `purchase_return_items`
--
ALTER TABLE `purchase_return_items`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_pri_return` (`tenant_id`,`return_id`),
  ADD KEY `fk_pri_pi` (`tenant_id`,`purchase_item_id`),
  ADD KEY `fk_pri_variant` (`tenant_id`,`variant_id`);

--
-- Indexes for table `register_shifts`
--
ALTER TABLE `register_shifts`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_register_shift_no` (`tenant_id`,`register_id`,`shift_no`),
  ADD UNIQUE KEY `uq_register_open_shift` (`tenant_id`,`register_id`,`open_key`),
  ADD KEY `idx_shifts_user` (`tenant_id`,`opened_by`,`status`),
  ADD KEY `fk_shifts_branch` (`tenant_id`,`branch_id`);

--
-- Indexes for table `roles`
--
ALTER TABLE `roles`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_roles_code` (`tenant_id`,`code`);

--
-- Indexes for table `role_permissions`
--
ALTER TABLE `role_permissions`
  ADD PRIMARY KEY (`tenant_id`,`role_id`,`permission_code`),
  ADD KEY `fk_role_perms_perm` (`permission_code`);

--
-- Indexes for table `sales`
--
ALTER TABLE `sales`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_sales_invoice` (`tenant_id`,`branch_id`,`invoice_no`),
  ADD UNIQUE KEY `uq_sales_idempotency` (`tenant_id`,`idempotency_key`),
  ADD KEY `idx_sales_branch_date` (`tenant_id`,`branch_id`,`business_date`),
  ADD KEY `idx_sales_date` (`tenant_id`,`business_date`,`sale_date`),
  ADD KEY `idx_sales_customer` (`tenant_id`,`customer_id`,`sale_date`),
  ADD KEY `idx_sales_cashier` (`tenant_id`,`cashier_id`,`business_date`),
  ADD KEY `idx_sales_due` (`tenant_id`,`customer_id`,`due_total`),
  ADD KEY `fk_sales_location` (`tenant_id`,`location_id`),
  ADD KEY `idx_sales_shift` (`tenant_id`,`register_shift_id`);

--
-- Indexes for table `sale_items`
--
ALTER TABLE `sale_items`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_sale_items_line` (`tenant_id`,`sale_id`,`line_no`),
  ADD KEY `idx_sale_items_variant` (`tenant_id`,`variant_id`);

--
-- Indexes for table `sale_returns`
--
ALTER TABLE `sale_returns`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_sale_returns_no` (`tenant_id`,`branch_id`,`return_no`),
  ADD UNIQUE KEY `uq_sale_returns_idem` (`tenant_id`,`idempotency_key`),
  ADD KEY `idx_sale_returns_sale` (`tenant_id`,`sale_id`),
  ADD KEY `idx_sale_returns_date` (`tenant_id`,`branch_id`,`business_date`),
  ADD KEY `fk_sale_returns_customer` (`tenant_id`,`customer_id`);

--
-- Indexes for table `sale_return_items`
--
ALTER TABLE `sale_return_items`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_sri_return` (`tenant_id`,`return_id`),
  ADD KEY `fk_sri_sale_item` (`tenant_id`,`sale_item_id`),
  ADD KEY `fk_sri_variant` (`tenant_id`,`variant_id`),
  ADD KEY `fk_sri_location` (`tenant_id`,`location_id`);

--
-- Indexes for table `stock_adjustments`
--
ALTER TABLE `stock_adjustments`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_adjustments_no` (`tenant_id`,`branch_id`,`adjustment_no`),
  ADD KEY `idx_adjustments_date` (`tenant_id`,`business_date`),
  ADD KEY `fk_adjustments_location` (`tenant_id`,`location_id`);

--
-- Indexes for table `stock_adjustment_items`
--
ALTER TABLE `stock_adjustment_items`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `idx_adj_items_adjustment` (`tenant_id`,`adjustment_id`),
  ADD KEY `fk_adj_items_variant` (`tenant_id`,`variant_id`);

--
-- Indexes for table `stock_balances`
--
ALTER TABLE `stock_balances`
  ADD PRIMARY KEY (`tenant_id`,`location_id`,`variant_id`),
  ADD KEY `idx_stock_balances_variant` (`tenant_id`,`variant_id`);

--
-- Indexes for table `stock_counts`
--
ALTER TABLE `stock_counts`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_counts_no` (`tenant_id`,`branch_id`,`count_no`),
  ADD KEY `idx_counts_status` (`tenant_id`,`status`,`created_at`),
  ADD KEY `fk_counts_location` (`tenant_id`,`location_id`),
  ADD KEY `fk_counts_category` (`tenant_id`,`category_id`);

--
-- Indexes for table `stock_count_items`
--
ALTER TABLE `stock_count_items`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_count_items` (`tenant_id`,`count_id`,`variant_id`),
  ADD KEY `fk_count_items_variant` (`tenant_id`,`variant_id`);

--
-- Indexes for table `stock_locations`
--
ALTER TABLE `stock_locations`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_stock_locations_code` (`tenant_id`,`code`),
  ADD KEY `fk_stock_locations_branch` (`tenant_id`,`branch_id`);

--
-- Indexes for table `stock_movements`
--
ALTER TABLE `stock_movements`
  ADD PRIMARY KEY (`tenant_id`,`variant_id`,`location_id`,`created_at`,`id`),
  ADD KEY `idx_movements_reference` (`tenant_id`,`reference_type`,`reference_id`),
  ADD KEY `idx_movements_date` (`tenant_id`,`business_date`);

--
-- Indexes for table `stock_transfers`
--
ALTER TABLE `stock_transfers`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_transfers_no` (`tenant_id`,`from_branch_id`,`transfer_no`),
  ADD KEY `idx_transfers_status` (`tenant_id`,`status`,`created_at`),
  ADD KEY `fk_transfers_to_branch` (`tenant_id`,`to_branch_id`),
  ADD KEY `fk_transfers_from` (`tenant_id`,`from_location_id`),
  ADD KEY `fk_transfers_to` (`tenant_id`,`to_location_id`);

--
-- Indexes for table `stock_transfer_items`
--
ALTER TABLE `stock_transfer_items`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_transfer_items` (`tenant_id`,`transfer_id`,`variant_id`),
  ADD KEY `fk_transfer_items_variant` (`tenant_id`,`variant_id`);

--
-- Indexes for table `subscriptions`
--
ALTER TABLE `subscriptions`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_subscriptions_tenant` (`tenant_id`),
  ADD KEY `idx_subscriptions_due` (`status`,`current_period_end`),
  ADD KEY `fk_subscriptions_plan` (`plan_id`),
  ADD KEY `fk_subscriptions_pending` (`pending_plan_id`);

--
-- Indexes for table `subscription_events`
--
ALTER TABLE `subscription_events`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD KEY `fk_sub_events_sub` (`tenant_id`,`subscription_id`);

--
-- Indexes for table `subscription_invoices`
--
ALTER TABLE `subscription_invoices`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_sub_invoices_no` (`invoice_no`),
  ADD KEY `fk_sub_invoices_plan` (`plan_id`);

--
-- Indexes for table `subscription_payment_claims`
--
ALTER TABLE `subscription_payment_claims`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_claims_txn` (`method`,`transaction_ref`),
  ADD KEY `fk_claims_invoice` (`tenant_id`,`invoice_id`);

--
-- Indexes for table `suppliers`
--
ALTER TABLE `suppliers`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_suppliers_code` (`tenant_id`,`code`),
  ADD KEY `idx_suppliers_name` (`tenant_id`,`name`),
  ADD KEY `idx_suppliers_due` (`tenant_id`,`current_balance`);

--
-- Indexes for table `tax_rates`
--
ALTER TABLE `tax_rates`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_tax_rates_name` (`tenant_id`,`name`);

--
-- Indexes for table `tenants`
--
ALTER TABLE `tenants`
  ADD PRIMARY KEY (`id`),
  ADD UNIQUE KEY `uq_tenants_slug` (`slug`);

--
-- Indexes for table `tenant_feature_overrides`
--
ALTER TABLE `tenant_feature_overrides`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_feature_overrides` (`tenant_id`,`feature_code`),
  ADD KEY `fk_overrides_feature` (`feature_code`);

--
-- Indexes for table `tenant_memberships`
--
ALTER TABLE `tenant_memberships`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_memberships_user` (`tenant_id`,`user_id`),
  ADD KEY `idx_memberships_user` (`user_id`,`status`);

--
-- Indexes for table `tenant_settings`
--
ALTER TABLE `tenant_settings`
  ADD PRIMARY KEY (`tenant_id`),
  ADD KEY `fk_settings_default_tax` (`tenant_id`,`default_tax_rate_id`);

--
-- Indexes for table `units`
--
ALTER TABLE `units`
  ADD PRIMARY KEY (`tenant_id`,`id`),
  ADD UNIQUE KEY `uq_units_short` (`tenant_id`,`short_name`);

--
-- Indexes for table `users`
--
ALTER TABLE `users`
  ADD PRIMARY KEY (`id`),
  ADD UNIQUE KEY `uq_users_phone` (`phone`),
  ADD UNIQUE KEY `uq_users_email` (`email`);

--
-- Indexes for table `user_sessions`
--
ALTER TABLE `user_sessions`
  ADD PRIMARY KEY (`id`),
  ADD UNIQUE KEY `uq_user_sessions_token` (`refresh_token_hash`),
  ADD KEY `idx_user_sessions_user` (`user_id`,`revoked_at`),
  ADD KEY `idx_user_sessions_family` (`family_id`),
  ADD KEY `fk_user_sessions_tenant` (`tenant_id`);

--
-- Indexes for table `_sqlx_migrations`
--
ALTER TABLE `_sqlx_migrations`
  ADD PRIMARY KEY (`version`);

--
-- Constraints for dumped tables
--

--
-- Constraints for table `account_transactions`
--
ALTER TABLE `account_transactions`
  ADD CONSTRAINT `fk_account_txn_account` FOREIGN KEY (`tenant_id`,`account_id`) REFERENCES `payment_accounts` (`tenant_id`, `id`);

--
-- Constraints for table `account_transfers`
--
ALTER TABLE `account_transfers`
  ADD CONSTRAINT `fk_acc_transfers_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_acc_transfers_from` FOREIGN KEY (`tenant_id`,`from_account_id`) REFERENCES `payment_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_acc_transfers_to` FOREIGN KEY (`tenant_id`,`to_account_id`) REFERENCES `payment_accounts` (`tenant_id`, `id`);

--
-- Constraints for table `bank_reconciliations`
--
ALTER TABLE `bank_reconciliations`
  ADD CONSTRAINT `fk_bank_rec_account` FOREIGN KEY (`tenant_id`,`account_id`) REFERENCES `payment_accounts` (`tenant_id`, `id`);

--
-- Constraints for table `bank_reconciliation_items`
--
ALTER TABLE `bank_reconciliation_items`
  ADD CONSTRAINT `fk_bank_rec_items_rec` FOREIGN KEY (`tenant_id`,`reconciliation_id`) REFERENCES `bank_reconciliations` (`tenant_id`, `id`) ON DELETE CASCADE,
  ADD CONSTRAINT `fk_bank_rec_items_txn` FOREIGN KEY (`tenant_id`,`account_transaction_id`) REFERENCES `account_transactions` (`tenant_id`, `id`);

--
-- Constraints for table `branches`
--
ALTER TABLE `branches`
  ADD CONSTRAINT `fk_branches_default_location` FOREIGN KEY (`tenant_id`,`default_location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_branches_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `brands`
--
ALTER TABLE `brands`
  ADD CONSTRAINT `fk_brands_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `cash_registers`
--
ALTER TABLE `cash_registers`
  ADD CONSTRAINT `fk_registers_account` FOREIGN KEY (`tenant_id`,`cash_account_id`) REFERENCES `payment_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_registers_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`);

--
-- Constraints for table `categories`
--
ALTER TABLE `categories`
  ADD CONSTRAINT `fk_categories_parent` FOREIGN KEY (`tenant_id`,`parent_id`) REFERENCES `categories` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_categories_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `customers`
--
ALTER TABLE `customers`
  ADD CONSTRAINT `fk_customers_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `document_sequences`
--
ALTER TABLE `document_sequences`
  ADD CONSTRAINT `fk_docseq_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_docseq_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `expenses`
--
ALTER TABLE `expenses`
  ADD CONSTRAINT `fk_expenses_account` FOREIGN KEY (`tenant_id`,`account_id`) REFERENCES `payment_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_expenses_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_expenses_category` FOREIGN KEY (`tenant_id`,`category_id`) REFERENCES `expense_categories` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_expenses_payment` FOREIGN KEY (`tenant_id`,`payment_id`) REFERENCES `payments` (`tenant_id`, `id`);

--
-- Constraints for table `expense_categories`
--
ALTER TABLE `expense_categories`
  ADD CONSTRAINT `fk_expense_categories_ledger` FOREIGN KEY (`tenant_id`,`ledger_account_id`) REFERENCES `ledger_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_expense_categories_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `files`
--
ALTER TABLE `files`
  ADD CONSTRAINT `fk_files_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `fiscal_periods`
--
ALTER TABLE `fiscal_periods`
  ADD CONSTRAINT `fk_fiscal_periods_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `goods_receipts`
--
ALTER TABLE `goods_receipts`
  ADD CONSTRAINT `fk_grn_location` FOREIGN KEY (`tenant_id`,`location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_grn_purchase` FOREIGN KEY (`tenant_id`,`purchase_id`) REFERENCES `purchases` (`tenant_id`, `id`);

--
-- Constraints for table `goods_receipt_items`
--
ALTER TABLE `goods_receipt_items`
  ADD CONSTRAINT `fk_grn_items_pi` FOREIGN KEY (`tenant_id`,`purchase_item_id`) REFERENCES `purchase_items` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_grn_items_receipt` FOREIGN KEY (`tenant_id`,`receipt_id`) REFERENCES `goods_receipts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_grn_items_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`);

--
-- Constraints for table `held_carts`
--
ALTER TABLE `held_carts`
  ADD CONSTRAINT `fk_held_carts_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`);

--
-- Constraints for table `idempotency_keys`
--
ALTER TABLE `idempotency_keys`
  ADD CONSTRAINT `fk_idempotency_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `income_categories`
--
ALTER TABLE `income_categories`
  ADD CONSTRAINT `fk_income_categories_ledger` FOREIGN KEY (`tenant_id`,`ledger_account_id`) REFERENCES `ledger_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_income_categories_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `invitations`
--
ALTER TABLE `invitations`
  ADD CONSTRAINT `fk_invitations_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`),
  ADD CONSTRAINT `fk_invitations_user` FOREIGN KEY (`accepted_by`) REFERENCES `users` (`id`);

--
-- Constraints for table `journal_entries`
--
ALTER TABLE `journal_entries`
  ADD CONSTRAINT `fk_journal_reversal` FOREIGN KEY (`tenant_id`,`reversal_of_id`) REFERENCES `journal_entries` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_journal_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `journal_lines`
--
ALTER TABLE `journal_lines`
  ADD CONSTRAINT `fk_journal_lines_account` FOREIGN KEY (`tenant_id`,`account_id`) REFERENCES `ledger_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_journal_lines_entry` FOREIGN KEY (`tenant_id`,`entry_id`) REFERENCES `journal_entries` (`tenant_id`, `id`);

--
-- Constraints for table `ledger_accounts`
--
ALTER TABLE `ledger_accounts`
  ADD CONSTRAINT `fk_ledger_parent` FOREIGN KEY (`tenant_id`,`parent_id`) REFERENCES `ledger_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_ledger_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `membership_roles`
--
ALTER TABLE `membership_roles`
  ADD CONSTRAINT `fk_mr_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_mr_membership` FOREIGN KEY (`tenant_id`,`membership_id`) REFERENCES `tenant_memberships` (`tenant_id`, `id`) ON DELETE CASCADE,
  ADD CONSTRAINT `fk_mr_role` FOREIGN KEY (`tenant_id`,`role_id`) REFERENCES `roles` (`tenant_id`, `id`);

--
-- Constraints for table `notifications`
--
ALTER TABLE `notifications`
  ADD CONSTRAINT `fk_notifications_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`),
  ADD CONSTRAINT `fk_notifications_user` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`);

--
-- Constraints for table `notification_deliveries`
--
ALTER TABLE `notification_deliveries`
  ADD CONSTRAINT `fk_notif_deliveries_notification` FOREIGN KEY (`tenant_id`,`notification_id`) REFERENCES `notifications` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_notif_deliveries_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `notification_preferences`
--
ALTER TABLE `notification_preferences`
  ADD CONSTRAINT `fk_notif_prefs_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`),
  ADD CONSTRAINT `fk_notif_prefs_user` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`);

--
-- Constraints for table `other_incomes`
--
ALTER TABLE `other_incomes`
  ADD CONSTRAINT `fk_other_incomes_account` FOREIGN KEY (`tenant_id`,`account_id`) REFERENCES `payment_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_other_incomes_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_other_incomes_category` FOREIGN KEY (`tenant_id`,`category_id`) REFERENCES `income_categories` (`tenant_id`, `id`);

--
-- Constraints for table `otp_codes`
--
ALTER TABLE `otp_codes`
  ADD CONSTRAINT `fk_otp_user` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE;

--
-- Constraints for table `outbox_events`
--
ALTER TABLE `outbox_events`
  ADD CONSTRAINT `fk_outbox_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `party_ledger_entries`
--
ALTER TABLE `party_ledger_entries`
  ADD CONSTRAINT `fk_party_ledger_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `payments`
--
ALTER TABLE `payments`
  ADD CONSTRAINT `fk_payments_account` FOREIGN KEY (`tenant_id`,`account_id`) REFERENCES `payment_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_payments_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_payments_method` FOREIGN KEY (`tenant_id`,`payment_method_id`) REFERENCES `payment_methods` (`tenant_id`, `id`);

--
-- Constraints for table `payment_accounts`
--
ALTER TABLE `payment_accounts`
  ADD CONSTRAINT `fk_payment_accounts_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_payment_accounts_ledger` FOREIGN KEY (`tenant_id`,`ledger_account_id`) REFERENCES `ledger_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_payment_accounts_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `payment_allocations`
--
ALTER TABLE `payment_allocations`
  ADD CONSTRAINT `fk_allocations_payment` FOREIGN KEY (`tenant_id`,`payment_id`) REFERENCES `payments` (`tenant_id`, `id`);

--
-- Constraints for table `payment_methods`
--
ALTER TABLE `payment_methods`
  ADD CONSTRAINT `fk_payment_methods_account` FOREIGN KEY (`tenant_id`,`default_account_id`) REFERENCES `payment_accounts` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_payment_methods_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `plan_features`
--
ALTER TABLE `plan_features`
  ADD CONSTRAINT `fk_plan_features_feature` FOREIGN KEY (`feature_code`) REFERENCES `features` (`code`),
  ADD CONSTRAINT `fk_plan_features_plan` FOREIGN KEY (`plan_id`) REFERENCES `plans` (`id`) ON DELETE CASCADE;

--
-- Constraints for table `plan_prices`
--
ALTER TABLE `plan_prices`
  ADD CONSTRAINT `fk_plan_prices_plan` FOREIGN KEY (`plan_id`) REFERENCES `plans` (`id`);

--
-- Constraints for table `platform_staff`
--
ALTER TABLE `platform_staff`
  ADD CONSTRAINT `fk_platform_staff_user` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`);

--
-- Constraints for table `products`
--
ALTER TABLE `products`
  ADD CONSTRAINT `fk_products_brand` FOREIGN KEY (`tenant_id`,`brand_id`) REFERENCES `brands` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_products_category` FOREIGN KEY (`tenant_id`,`category_id`) REFERENCES `categories` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_products_image` FOREIGN KEY (`tenant_id`,`image_file_id`) REFERENCES `files` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_products_tax` FOREIGN KEY (`tenant_id`,`tax_rate_id`) REFERENCES `tax_rates` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_products_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`),
  ADD CONSTRAINT `fk_products_unit` FOREIGN KEY (`tenant_id`,`base_unit_id`) REFERENCES `units` (`tenant_id`, `id`);

--
-- Constraints for table `product_barcodes`
--
ALTER TABLE `product_barcodes`
  ADD CONSTRAINT `fk_barcodes_unit` FOREIGN KEY (`tenant_id`,`unit_id`) REFERENCES `units` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_barcodes_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`) ON DELETE CASCADE;

--
-- Constraints for table `product_price_history`
--
ALTER TABLE `product_price_history`
  ADD CONSTRAINT `fk_price_history_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`);

--
-- Constraints for table `purchases`
--
ALTER TABLE `purchases`
  ADD CONSTRAINT `fk_purchases_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_purchases_location` FOREIGN KEY (`tenant_id`,`location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_purchases_supplier` FOREIGN KEY (`tenant_id`,`supplier_id`) REFERENCES `suppliers` (`tenant_id`, `id`);

--
-- Constraints for table `purchase_items`
--
ALTER TABLE `purchase_items`
  ADD CONSTRAINT `fk_purchase_items_purchase` FOREIGN KEY (`tenant_id`,`purchase_id`) REFERENCES `purchases` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_purchase_items_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`);

--
-- Constraints for table `purchase_returns`
--
ALTER TABLE `purchase_returns`
  ADD CONSTRAINT `fk_pr_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_pr_location` FOREIGN KEY (`tenant_id`,`location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_pr_purchase` FOREIGN KEY (`tenant_id`,`purchase_id`) REFERENCES `purchases` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_pr_supplier` FOREIGN KEY (`tenant_id`,`supplier_id`) REFERENCES `suppliers` (`tenant_id`, `id`);

--
-- Constraints for table `purchase_return_items`
--
ALTER TABLE `purchase_return_items`
  ADD CONSTRAINT `fk_pri_pi` FOREIGN KEY (`tenant_id`,`purchase_item_id`) REFERENCES `purchase_items` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_pri_return` FOREIGN KEY (`tenant_id`,`return_id`) REFERENCES `purchase_returns` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_pri_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`);

--
-- Constraints for table `register_shifts`
--
ALTER TABLE `register_shifts`
  ADD CONSTRAINT `fk_shifts_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_shifts_register` FOREIGN KEY (`tenant_id`,`register_id`) REFERENCES `cash_registers` (`tenant_id`, `id`);

--
-- Constraints for table `roles`
--
ALTER TABLE `roles`
  ADD CONSTRAINT `fk_roles_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `role_permissions`
--
ALTER TABLE `role_permissions`
  ADD CONSTRAINT `fk_role_perms_perm` FOREIGN KEY (`permission_code`) REFERENCES `permissions` (`code`),
  ADD CONSTRAINT `fk_role_perms_role` FOREIGN KEY (`tenant_id`,`role_id`) REFERENCES `roles` (`tenant_id`, `id`) ON DELETE CASCADE;

--
-- Constraints for table `sales`
--
ALTER TABLE `sales`
  ADD CONSTRAINT `fk_sales_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_sales_customer` FOREIGN KEY (`tenant_id`,`customer_id`) REFERENCES `customers` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_sales_location` FOREIGN KEY (`tenant_id`,`location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`);

--
-- Constraints for table `sale_items`
--
ALTER TABLE `sale_items`
  ADD CONSTRAINT `fk_sale_items_sale` FOREIGN KEY (`tenant_id`,`sale_id`) REFERENCES `sales` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_sale_items_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`);

--
-- Constraints for table `sale_returns`
--
ALTER TABLE `sale_returns`
  ADD CONSTRAINT `fk_sale_returns_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_sale_returns_customer` FOREIGN KEY (`tenant_id`,`customer_id`) REFERENCES `customers` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_sale_returns_sale` FOREIGN KEY (`tenant_id`,`sale_id`) REFERENCES `sales` (`tenant_id`, `id`);

--
-- Constraints for table `sale_return_items`
--
ALTER TABLE `sale_return_items`
  ADD CONSTRAINT `fk_sri_location` FOREIGN KEY (`tenant_id`,`location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_sri_return` FOREIGN KEY (`tenant_id`,`return_id`) REFERENCES `sale_returns` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_sri_sale_item` FOREIGN KEY (`tenant_id`,`sale_item_id`) REFERENCES `sale_items` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_sri_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`);

--
-- Constraints for table `stock_adjustments`
--
ALTER TABLE `stock_adjustments`
  ADD CONSTRAINT `fk_adjustments_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_adjustments_location` FOREIGN KEY (`tenant_id`,`location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`);

--
-- Constraints for table `stock_adjustment_items`
--
ALTER TABLE `stock_adjustment_items`
  ADD CONSTRAINT `fk_adj_items_adjustment` FOREIGN KEY (`tenant_id`,`adjustment_id`) REFERENCES `stock_adjustments` (`tenant_id`, `id`) ON DELETE CASCADE,
  ADD CONSTRAINT `fk_adj_items_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`);

--
-- Constraints for table `stock_balances`
--
ALTER TABLE `stock_balances`
  ADD CONSTRAINT `fk_stock_balances_location` FOREIGN KEY (`tenant_id`,`location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_stock_balances_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`),
  ADD CONSTRAINT `fk_stock_balances_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`);

--
-- Constraints for table `stock_counts`
--
ALTER TABLE `stock_counts`
  ADD CONSTRAINT `fk_counts_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_counts_category` FOREIGN KEY (`tenant_id`,`category_id`) REFERENCES `categories` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_counts_location` FOREIGN KEY (`tenant_id`,`location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`);

--
-- Constraints for table `stock_count_items`
--
ALTER TABLE `stock_count_items`
  ADD CONSTRAINT `fk_count_items_count` FOREIGN KEY (`tenant_id`,`count_id`) REFERENCES `stock_counts` (`tenant_id`, `id`) ON DELETE CASCADE,
  ADD CONSTRAINT `fk_count_items_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`);

--
-- Constraints for table `stock_locations`
--
ALTER TABLE `stock_locations`
  ADD CONSTRAINT `fk_stock_locations_branch` FOREIGN KEY (`tenant_id`,`branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_stock_locations_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `stock_transfers`
--
ALTER TABLE `stock_transfers`
  ADD CONSTRAINT `fk_transfers_from` FOREIGN KEY (`tenant_id`,`from_location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_transfers_from_branch` FOREIGN KEY (`tenant_id`,`from_branch_id`) REFERENCES `branches` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_transfers_to` FOREIGN KEY (`tenant_id`,`to_location_id`) REFERENCES `stock_locations` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_transfers_to_branch` FOREIGN KEY (`tenant_id`,`to_branch_id`) REFERENCES `branches` (`tenant_id`, `id`);

--
-- Constraints for table `stock_transfer_items`
--
ALTER TABLE `stock_transfer_items`
  ADD CONSTRAINT `fk_transfer_items_transfer` FOREIGN KEY (`tenant_id`,`transfer_id`) REFERENCES `stock_transfers` (`tenant_id`, `id`) ON DELETE CASCADE,
  ADD CONSTRAINT `fk_transfer_items_variant` FOREIGN KEY (`tenant_id`,`variant_id`) REFERENCES `product_variants` (`tenant_id`, `id`);

--
-- Constraints for table `subscriptions`
--
ALTER TABLE `subscriptions`
  ADD CONSTRAINT `fk_subscriptions_pending` FOREIGN KEY (`pending_plan_id`) REFERENCES `plans` (`id`),
  ADD CONSTRAINT `fk_subscriptions_plan` FOREIGN KEY (`plan_id`) REFERENCES `plans` (`id`),
  ADD CONSTRAINT `fk_subscriptions_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `subscription_events`
--
ALTER TABLE `subscription_events`
  ADD CONSTRAINT `fk_sub_events_sub` FOREIGN KEY (`tenant_id`,`subscription_id`) REFERENCES `subscriptions` (`tenant_id`, `id`);

--
-- Constraints for table `subscription_invoices`
--
ALTER TABLE `subscription_invoices`
  ADD CONSTRAINT `fk_sub_invoices_plan` FOREIGN KEY (`plan_id`) REFERENCES `plans` (`id`),
  ADD CONSTRAINT `fk_sub_invoices_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `subscription_payment_claims`
--
ALTER TABLE `subscription_payment_claims`
  ADD CONSTRAINT `fk_claims_invoice` FOREIGN KEY (`tenant_id`,`invoice_id`) REFERENCES `subscription_invoices` (`tenant_id`, `id`);

--
-- Constraints for table `suppliers`
--
ALTER TABLE `suppliers`
  ADD CONSTRAINT `fk_suppliers_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `tax_rates`
--
ALTER TABLE `tax_rates`
  ADD CONSTRAINT `fk_tax_rates_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `tenant_feature_overrides`
--
ALTER TABLE `tenant_feature_overrides`
  ADD CONSTRAINT `fk_overrides_feature` FOREIGN KEY (`feature_code`) REFERENCES `features` (`code`),
  ADD CONSTRAINT `fk_overrides_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `tenant_memberships`
--
ALTER TABLE `tenant_memberships`
  ADD CONSTRAINT `fk_memberships_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`),
  ADD CONSTRAINT `fk_memberships_user` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`);

--
-- Constraints for table `tenant_settings`
--
ALTER TABLE `tenant_settings`
  ADD CONSTRAINT `fk_settings_default_tax` FOREIGN KEY (`tenant_id`,`default_tax_rate_id`) REFERENCES `tax_rates` (`tenant_id`, `id`),
  ADD CONSTRAINT `fk_tenant_settings_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `units`
--
ALTER TABLE `units`
  ADD CONSTRAINT `fk_units_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`);

--
-- Constraints for table `user_sessions`
--
ALTER TABLE `user_sessions`
  ADD CONSTRAINT `fk_user_sessions_tenant` FOREIGN KEY (`tenant_id`) REFERENCES `tenants` (`id`),
  ADD CONSTRAINT `fk_user_sessions_user` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE;
COMMIT;

/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
