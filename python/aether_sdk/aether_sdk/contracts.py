# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Aether contract discovery helpers.

Phase 0: resolve contract paths and surface the initial catalog.
Phase 2: extend with dynamic discovery from the offline registry.
"""

from __future__ import annotations

from .validation import CONTRACT_MANIFESTS, contract_path

__all__ = ["CONTRACT_MANIFESTS", "contract_path", "list_contracts"]


def list_contracts() -> list[str]:
    """Return the contract ids known to this SDK build."""
    return list(CONTRACT_MANIFESTS.keys())
