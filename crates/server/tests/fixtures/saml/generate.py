#!/usr/bin/env python3
"""Generates the signed SAML fixtures of `tests/saml_api.rs`.

Responses shaped like those of Entra ID, Okta and Keycloak, plus the hostile
variants the tests replay. Keys are generated in memory and thrown away: only
the public certificates and the signed documents are written, so the tests
never sign anything.

    python3 -m venv venv && venv/bin/pip install signxml
    venv/bin/python generate.py

Every timestamp is fixed (the tests pin the validation clock to
2026-10-26T10:01:00Z) and so is `InResponseTo` (the tests pin the request ID).
"""

import base64
import datetime
import pathlib

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID
from lxml import etree
from signxml import XMLSigner, methods
from signxml.algorithms import CanonicalizationMethod, DigestAlgorithm, SignatureMethod

OUT = pathlib.Path(__file__).resolve().parent

SAMLP = "urn:oasis:names:tc:SAML:2.0:protocol"
SAML = "urn:oasis:names:tc:SAML:2.0:assertion"
DS = "http://www.w3.org/2000/09/xmldsig#"
XS = "http://www.w3.org/2001/XMLSchema"
XSI = "http://www.w3.org/2001/XMLSchema-instance"

ACS = "http://localhost:8080/api/auth/saml/acs"
SP_ENTITY = "https://monit.example.test/saml"
REQUEST_ID = "_dm-test-request-0001"

ISSUE = "2026-10-26T10:00:00Z"
NOT_BEFORE = "2026-10-26T09:55:00Z"
NOT_AFTER = "2026-10-26T10:10:00Z"
CONFIRM_UNTIL = "2026-10-26T10:05:00Z"
AUTHN = "2026-10-26T09:59:30Z"
SESSION_UNTIL = "2026-10-26T18:00:00Z"

PERSISTENT = "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent"
EMAIL = "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress"
UNSPECIFIED = "urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified"
BASIC = "urn:oasis:names:tc:SAML:2.0:attrname-format:basic"
URI = "urn:oasis:names:tc:SAML:2.0:attrname-format:uri"


def keypair(name):
    key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    subject = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, name)])
    cert = (
        x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(subject)
        .public_key(key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(datetime.datetime(2026, 1, 1, tzinfo=datetime.timezone.utc))
        .not_valid_after(datetime.datetime(2046, 1, 1, tzinfo=datetime.timezone.utc))
        .sign(key, hashes.SHA256())
    )
    key_pem = key.private_bytes(
        serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8, serialization.NoEncryption()
    )
    return key_pem, cert.public_bytes(serialization.Encoding.PEM)


class LegacySigner(XMLSigner):
    """SHA-1, refused by signxml by default: only to prove that we refuse it too."""

    def check_deprecated_methods(self):
        pass


def sign(element, key, cert, ds_prefix="ds", sha1=False):
    """Enveloped signature of `element`, placed after its Issuer; returns bytes.

    Documents are assembled as text from here on: moving a signed element into
    another lxml tree would rewrite its namespace prefixes, and exclusive
    canonicalization is sensitive to them.
    """
    if isinstance(element, bytes):
        element = etree.fromstring(element)
    cls = LegacySigner if sha1 else XMLSigner
    signer = cls(
        method=methods.enveloped,
        signature_algorithm=SignatureMethod.RSA_SHA1 if sha1 else SignatureMethod.RSA_SHA256,
        digest_algorithm=DigestAlgorithm.SHA1 if sha1 else DigestAlgorithm.SHA256,
        c14n_algorithm=CanonicalizationMethod.EXCLUSIVE_XML_CANONICALIZATION_1_0,
    )
    signer.namespaces = {ds_prefix: DS}
    issuer = element.find(f"{{{SAML}}}Issuer")
    placeholder = etree.Element(f"{{{DS}}}Signature", nsmap={ds_prefix: DS}, Id="placeholder")
    issuer.addnext(placeholder)
    signed = signer.sign(element, key=key, cert=cert, reference_uri=element.get("ID"), id_attribute="ID")
    return etree.tostring(signed)


def assertion(
    *,
    prefix,
    issuer,
    assertion_id,
    name_id,
    name_format,
    attributes,
    in_response_to=REQUEST_ID,
    audience=SP_ENTITY,
    recipient=ACS,
    typed_values=False,
):
    """An Assertion. `prefix=None` puts SAML in the default namespace (Entra ID)."""
    nsmap = {prefix: SAML}
    if typed_values:
        nsmap.update({"xs": XS, "xsi": XSI})
    q = lambda tag: f"{{{SAML}}}{tag}"  # noqa: E731
    root = etree.Element(q("Assertion"), nsmap=nsmap, ID=assertion_id, Version="2.0", IssueInstant=ISSUE)
    etree.SubElement(root, q("Issuer")).text = issuer
    subject = etree.SubElement(root, q("Subject"))
    etree.SubElement(subject, q("NameID"), Format=name_format).text = name_id
    confirmation = etree.SubElement(
        subject, q("SubjectConfirmation"), Method="urn:oasis:names:tc:SAML:2.0:cm:bearer"
    )
    data = {"NotOnOrAfter": CONFIRM_UNTIL, "Recipient": recipient}
    if in_response_to:
        data["InResponseTo"] = in_response_to
    etree.SubElement(confirmation, q("SubjectConfirmationData"), **data)
    conditions = etree.SubElement(root, q("Conditions"), NotBefore=NOT_BEFORE, NotOnOrAfter=NOT_AFTER)
    restriction = etree.SubElement(conditions, q("AudienceRestriction"))
    etree.SubElement(restriction, q("Audience")).text = audience
    statement = etree.SubElement(root, q("AttributeStatement"))
    for name, values, fmt in attributes:
        attribute = etree.SubElement(statement, q("Attribute"), Name=name)
        if fmt:
            attribute.set("NameFormat", fmt)
        for value in values:
            node = etree.SubElement(attribute, q("AttributeValue"))
            if typed_values:
                node.set(f"{{{XSI}}}type", "xs:string")
            node.text = value
    authn = etree.SubElement(
        root, q("AuthnStatement"), AuthnInstant=AUTHN, SessionIndex=f"{assertion_id}-session",
        SessionNotOnOrAfter=SESSION_UNTIL,
    )
    context = etree.SubElement(authn, q("AuthnContext"))
    etree.SubElement(context, q("AuthnContextClassRef")).text = (
        "urn:oasis:names:tc:SAML:2.0:ac:classes:PasswordProtectedTransport"
    )
    return root


def response(*, prefixes, issuer, response_id, assertions, in_response_to=REQUEST_ID, destination=ACS):
    """The Response as bytes; `assertions` are bytes (signed) or elements."""
    protocol, saml = prefixes
    root = etree.Element(
        f"{{{SAMLP}}}Response", nsmap={protocol: SAMLP, saml: SAML}, ID=response_id, Version="2.0",
        IssueInstant=ISSUE, Destination=destination,
    )
    if in_response_to:
        root.set("InResponseTo", in_response_to)
    etree.SubElement(root, f"{{{SAML}}}Issuer").text = issuer
    status = etree.SubElement(root, f"{{{SAMLP}}}Status")
    etree.SubElement(status, f"{{{SAMLP}}}StatusCode", Value="urn:oasis:names:tc:SAML:2.0:status:Success")
    text = etree.tostring(root)
    closing = f"</{protocol}:Response>".encode()
    body = b"".join(item if isinstance(item, bytes) else etree.tostring(item) for item in assertions)
    return text[: -len(closing)] + body + closing


def write(name, data):
    (OUT / f"{name}.xml").write_bytes(b'<?xml version="1.0" encoding="UTF-8"?>\n' + data)


def write_cert(name, cert):
    (OUT / f"{name}-cert.pem").write_bytes(cert)


def main():
    entra_key, entra_cert = keypair("Microsoft Azure Federated SSO Certificate (test)")
    okta_key, okta_cert = keypair("dumbmonit-okta-test")
    kc_key, kc_cert = keypair("lab-realm (test)")
    rogue_key, _ = keypair("not-the-idp")
    write_cert("entra", entra_cert)
    write_cert("okta", okta_cert)
    write_cert("keycloak", kc_cert)

    # --- Entra ID : seule l'assertion est signée (réglage par défaut), espace de
    # noms SAML par défaut sur l'assertion, revendications sous forme d'URI.
    entra_issuer = "https://sts.windows.net/9188040d-6c67-4c5b-b112-36a304b66dad/"

    def entra_assertion(**overrides):
        args = dict(
            prefix=None,
            issuer=entra_issuer,
            assertion_id="_6a2e9c3b-entra-assertion-0001",
            name_id="alice@contoso.example",
            name_format=EMAIL,
            attributes=[
                ("http://schemas.microsoft.com/identity/claims/tenantid",
                 ["9188040d-6c67-4c5b-b112-36a304b66dad"], None),
                ("http://schemas.microsoft.com/identity/claims/objectidentifier",
                 ["4d1f2b7a-0c3e-4f5a-9b8c-7d6e5f4a3b2c"], None),
                ("http://schemas.microsoft.com/identity/claims/displayname", ["Alice Martin"], None),
                ("http://schemas.microsoft.com/ws/2008/06/identity/claims/groups",
                 ["0b6a1f3e-2c4d-4e5f-8a9b-operators000", "7c8d9e0f-1a2b-4c3d-9e8f-everyone0000"], None),
                ("http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
                 ["alice@contoso.example"], None),
                ("http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name",
                 ["alice@contoso.example"], None),
            ],
        )
        args.update(overrides)
        return assertion(**args)

    def entra_response(assertions, response_id="_entra-response-0001", **overrides):
        return response(
            prefixes=("samlp", "saml"), issuer=entra_issuer, response_id=response_id,
            assertions=assertions, **overrides,
        )

    signed = sign(entra_assertion(), entra_key, entra_cert)
    write("entra-response", entra_response([signed]))

    # Sans signature du tout.
    write("unsigned-response", entra_response([entra_assertion()], response_id="_unsigned-0001"))

    # Signée puis modifiée : l'adresse change après la signature.
    tampered = signed.replace(
        b">alice@contoso.example</AttributeValue>", b">mallory@contoso.example</AttributeValue>"
    )
    assert tampered != signed
    write("tampered-response", entra_response([tampered], response_id="_tampered-0001"))

    # Signée par une clé que l'IdP n'a pas déclarée.
    write("rogue-key-response", entra_response(
        [sign(entra_assertion(), rogue_key, entra_cert)], response_id="_rogue-0001"))

    # XSW 1 : une assertion non signée au nom de Mallory, posée avant l'assertion
    # signée (intacte) d'Alice.
    evil = entra_assertion(assertion_id="_evil-assertion-0001", name_id="mallory@contoso.example")
    write("xsw-sibling-response", entra_response(
        [evil, signed], response_id="_xsw-sibling-0001"))

    # XSW 2 : l'assertion de Mallory enveloppe l'assertion signée d'Alice dans
    # son `Advice` ; la référence `#ID` signée se résout toujours.
    wrapper = entra_assertion(assertion_id="_evil-wrapper-0001", name_id="mallory@contoso.example")
    wrapper = etree.tostring(wrapper)
    wrapper = wrapper.replace(b"</Conditions>", b"</Conditions><Advice>" + signed + b"</Advice>", 1)
    assert signed in wrapper
    write("xsw-advice-response", entra_response([wrapper], response_id="_xsw-advice-0001"))

    # Signées, mais pour un autre service, une autre adresse, une autre requête.
    write("wrong-audience-response", entra_response(
        [sign(entra_assertion(assertion_id="_aud-0001", audience="https://other.example/sp"),
              entra_key, entra_cert)], response_id="_aud-response-0001"))
    write("wrong-destination-response", entra_response(
        [sign(entra_assertion(assertion_id="_dest-0001", recipient="https://evil.example/acs"),
              entra_key, entra_cert)],
        response_id="_dest-response-0001", destination="https://evil.example/acs"))
    write("unknown-request-response", entra_response(
        [sign(entra_assertion(assertion_id="_irt-0001", in_response_to="_someone-elses-request"),
              entra_key, entra_cert)],
        response_id="_irt-response-0001", in_response_to="_someone-elses-request"))

    # RSA-SHA1 et condensé SHA-1.
    write("sha1-response", entra_response(
        [sign(entra_assertion(assertion_id="_sha1-0001"), entra_key, entra_cert, sha1=True)],
        response_id="_sha1-response-0001"))

    # Initiée par l'IdP : aucune requête, donc aucun `InResponseTo`.
    write("idp-initiated-response", entra_response(
        [sign(entra_assertion(assertion_id="_idp-init-0001", in_response_to=None),
              entra_key, entra_cert)],
        response_id="_idp-init-response-0001", in_response_to=None))

    # --- Okta : réponse et assertion signées, préfixes saml2p/saml2, valeurs
    # typées xs:string.
    okta_issuer = "http://www.okta.com/exk8dumbmonittest0h7"
    okta_assertion = assertion(
        prefix="saml2",
        issuer=okta_issuer,
        assertion_id="id17045812345678901234567890",
        name_id="bob@okta.example",
        name_format=UNSPECIFIED,
        attributes=[
            ("email", ["bob@okta.example"], UNSPECIFIED),
            ("displayName", ["Bob Durand"], UNSPECIFIED),
            ("groups", ["Everyone", "dumbmonit-operators"], UNSPECIFIED),
        ],
        typed_values=True,
    )
    okta_signed = sign(okta_assertion, okta_key, okta_cert)
    okta = response(
        prefixes=("saml2p", "saml2"), issuer=okta_issuer, response_id="id17045812345678900000000001",
        assertions=[okta_signed],
    )
    write("okta-response", sign(okta, okta_key, okta_cert))

    # --- Keycloak : réponse et assertion signées, préfixe dsig, NameID
    # persistant, groupes en chemin complet.
    kc_issuer = "https://keycloak.example.test/realms/lab"
    kc_assertion = assertion(
        prefix="saml",
        issuer=kc_issuer,
        assertion_id="ID_5b1f0e2c-3d4a-4b5c-8d9e-0f1a2b3c4d5e",
        name_id="G-1c2d3e4f-5a6b-7c8d-9e0f-carolkeycloak",
        name_format=PERSISTENT,
        attributes=[
            ("email", ["carol@kc.example"], BASIC),
            ("firstName", ["Carol"], BASIC),
            ("groups", ["/dumbmonit-admins", "/ops"], BASIC),
            ("Role", ["default-roles-lab", "offline_access"], BASIC),
        ],
    )
    kc_signed = sign(kc_assertion, kc_key, kc_cert, ds_prefix="dsig")
    kc = response(
        prefixes=("samlp", "saml"), issuer=kc_issuer, response_id="ID_9e8d7c6b-response-keycloak",
        assertions=[kc_signed],
    )
    write("keycloak-response", sign(kc, kc_key, kc_cert, ds_prefix="dsig"))

    # Métadonnées d'IdP façon Okta, pour l'import.
    body = "".join(okta_cert.decode().strip().splitlines()[1:-1])
    metadata = f"""<?xml version="1.0" encoding="UTF-8"?>
<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" entityID="{okta_issuer}">
  <md:IDPSSODescriptor WantAuthnRequestsSigned="false" protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <md:KeyDescriptor use="signing">
      <ds:KeyInfo xmlns:ds="http://www.w3.org/2000/09/xmldsig#">
        <ds:X509Data>
          <ds:X509Certificate>{body}</ds:X509Certificate>
        </ds:X509Data>
      </ds:KeyInfo>
    </md:KeyDescriptor>
    <md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified</md:NameIDFormat>
    <md:SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="https://dumbmonit-test.okta.example/app/dumbmonit/exk8dumbmonittest0h7/sso/saml"/>
    <md:SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" Location="https://dumbmonit-test.okta.example/app/dumbmonit/exk8dumbmonittest0h7/sso/saml"/>
  </md:IDPSSODescriptor>
</md:EntityDescriptor>
"""
    (OUT / "okta-metadata.xml").write_text(metadata)
    assert base64.b64decode(body)


if __name__ == "__main__":
    main()
