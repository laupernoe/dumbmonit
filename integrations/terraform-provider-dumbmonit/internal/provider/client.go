package provider

import (
	"bytes"
	"context"
	"crypto/tls"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// errNotFound : l'objet n'existe plus côté serveur (supprimé à la main).
var errNotFound = errors.New("not found")

// apiClient parle à l'API REST de DumbMonit avec un jeton d'API de portée write.
type apiClient struct {
	base  string
	token string
	http  *http.Client
}

func newClient(base, token string, insecure bool) *apiClient {
	transport := http.DefaultTransport.(*http.Transport).Clone()
	if insecure {
		transport.TLSClientConfig = &tls.Config{InsecureSkipVerify: true} //nolint:gosec // demandé explicitement
	}
	return &apiClient{
		base:  strings.TrimRight(base, "/"),
		token: token,
		http:  &http.Client{Timeout: 60 * time.Second, Transport: transport},
	}
}

// do envoie une requête JSON et décode la réponse dans out (si non nul).
func (c *apiClient) do(ctx context.Context, method, path string, in, out any) error {
	var body io.Reader
	if in != nil {
		data, err := json.Marshal(in)
		if err != nil {
			return err
		}
		body = bytes.NewReader(data)
	}
	req, err := http.NewRequestWithContext(ctx, method, c.base+path, body)
	if err != nil {
		return err
	}
	req.Header.Set("Authorization", "Bearer "+c.token)
	req.Header.Set("Accept", "application/json")
	req.Header.Set("X-Requested-With", "DumbMonit")
	if in != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	resp, err := c.http.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(io.LimitReader(resp.Body, 16<<20))
	if err != nil {
		return err
	}
	if resp.StatusCode == http.StatusNotFound {
		return errNotFound
	}
	if resp.StatusCode >= 300 {
		var apiErr struct {
			Error string `json:"error"`
		}
		message := strings.TrimSpace(string(data))
		if json.Unmarshal(data, &apiErr) == nil && apiErr.Error != "" {
			message = apiErr.Error
		}
		return fmt.Errorf("%s %s: HTTP %d: %s", method, path, resp.StatusCode, message)
	}
	if out != nil && len(data) > 0 {
		return json.Unmarshal(data, out)
	}
	return nil
}
