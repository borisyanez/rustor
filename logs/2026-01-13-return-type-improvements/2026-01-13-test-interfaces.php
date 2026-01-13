<?php

use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\RequestInterface;
use Psr\Http\Message\UriInterface;
use Psr\Http\Message\StreamInterface;
use Psr\Http\Client\ClientInterface;
use Doctrine\Common\Collections\Collection;
use Doctrine\Common\Collections\ArrayCollection;

// Test 1: PSR-7 ResponseInterface
function getResponse(): ResponseInterface {
    return new Response();  // Should NOT error
}

// Test 2: PSR-7 RequestInterface
function getRequest(): RequestInterface {
    return new Request();  // Should NOT error
}

// Test 3: PSR-7 UriInterface
function getUri(): UriInterface {
    return new Uri();  // Should NOT error
}

// Test 4: PSR-7 StreamInterface
function getStream(): StreamInterface {
    return new Stream();  // Should NOT error
}

// Test 5: PSR-18 ClientInterface
function getClient(): ClientInterface {
    return new Client();  // Should NOT error
}

// Test 6: Doctrine Collection
function getCollection(): Collection {
    return new ArrayCollection();  // Should NOT error
}

// Test 7: These SHOULD still error (wrong type)
function wrongInterface(): ResponseInterface {
    return new Request();  // SHOULD error - Request is not Response
}

class Response {}
class Request {}
class Uri {}
class Stream {}
class Client {}
